// cuda/base_neuron.cu

#include "base_neuron.h"
#include <vector>
#include <cmath>
#include <cstdint>
#include <stdio.h>

#ifdef USE_CUDA
#include <curand_kernel.h>
#endif

// Simple PRNG for Mock CPU (Xorshift)
static float get_random_float_01(uint32_t* seed) {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    return (float)(*seed % 10000) / 10000.0f;
}

// Gaussian Random Generation
static __device__ __host__ float get_gaussian(uint32_t* seed, float mu, float sigma) {
#ifdef USE_CUDA
    // On GPU: Use NVIDIA's native high-performance RNG
    curandState_t state;
    curand_init(*seed, 0, 0, &state);
    return mu + sigma * curand_normal(&state);
#else
    // On CPU: Use refined Box-Muller transform
    float u1 = get_random_float_01(seed);
    float u2 = get_random_float_01(seed);
    if (u1 < 0.00001f) u1 = 0.00001f;
    float z = (float)sqrt(-2.0 * log(u1)) * (float)cos(2.0 * 3.14159265 * u2);
    return mu + sigma * z;
#endif
}

// --- HELPER MACROS FOR FIXED-POINT ---
static inline __device__ __host__ uint16_t pack_voltage(float v) {
    float shifted = v + VOLTAGE_SHIFT;
    if (shifted < 0.0f) shifted = 0.0f;
    if (shifted > MAX_U16_FLOAT / VOLTAGE_SCALE) shifted = MAX_U16_FLOAT / VOLTAGE_SCALE;
    return (uint16_t)(shifted * VOLTAGE_SCALE);
}


static inline __device__ __host__ float unpack_voltage(uint16_t v) {
    return ((float)v / VOLTAGE_SCALE) - VOLTAGE_SHIFT;
}

#ifdef USE_CUDA
__device__ void atomic_add_packed_voltage(uint16_t* address, float voltage_impact) {
#if __CUDA_ARCH__ >= 700
    // Warp-level reduction for atomic CAS spin-lock mitigation (Phase 31 VRAM Diet)
    // Find all threads in the warp writing to the EXACT SAME address
    uint32_t active = __activemask();
    int lane_id = threadIdx.x % 32;
    
    // Simpler warp reduction without complex masking: just use atomicAdd to shared memory if possible, 
    // but since address is global, we can't easily map it to shared.
    // Let's stick to the simplest standard warp-aggregate for global atomics:
    
    float thread_val = voltage_impact;
    for (int i = 1; i < 32; i++) {
        int src_lane = (lane_id + i) % 32;
        unsigned long long src_addr = __shfl_sync(active, (unsigned long long)address, src_lane);
        float src_val = __shfl_sync(active, thread_val, src_lane);
        if (src_addr == (unsigned long long)address && src_lane > lane_id) {
            // We are a lower lane, we absorb the higher lane's value
            voltage_impact += src_val;
        }
        if (src_addr == (unsigned long long)address && src_lane < lane_id) {
            // A lower lane absorbed our value, we bail out
            voltage_impact = 0.0f;
            break;
        }
    }
    
    // Only the lowest lane for this address performs the atomic
    // If voltage_impact became exactly 0.0f (either because we are a higher lane, 
    // or because the sum was exactly 0), we can safely skip the atomic.
    if (voltage_impact != 0.0f) {
        uint16_t assumed, old = *address;
        do {
            assumed = old;
            float current_v = unpack_voltage(assumed);
            current_v += voltage_impact;
            current_v = fmaxf(-100.0f, fminf(current_v, 150.0f)); // Saturation limit (Spec 2.2)
            uint16_t new_val = pack_voltage(current_v);
            old = atomicCAS(address, assumed, new_val);
        } while (assumed != old);
    }
#else
    uint16_t assumed, old = *address;
    do {
        assumed = old;
        float current_v = unpack_voltage(assumed);
        current_v += voltage_impact;
        current_v = fmaxf(-100.0f, fminf(current_v, 150.0f)); // Saturation limit (Spec 2.2)
        uint16_t new_val = pack_voltage(current_v);
        old = atomicCAS(address, assumed, new_val);
    } while (assumed != old);
#endif
}
#else
#include <atomic>
static inline void atomic_add_packed_voltage(uint16_t* address, float voltage_impact) {
    std::atomic<uint16_t>* atomic_addr = reinterpret_cast<std::atomic<uint16_t>*>(address);
    uint16_t old = atomic_addr->load(std::memory_order_relaxed);
    while (true) {
        float current_v = unpack_voltage(old);
        current_v += voltage_impact;
        current_v = current_v > 150.0f ? 150.0f : (current_v < -100.0f ? -100.0f : current_v); // Saturation limit (Spec 2.2)
        uint16_t new_val = pack_voltage(current_v);
        if (atomic_addr->compare_exchange_weak(old, new_val, std::memory_order_relaxed)) {
            break;
        }
    }
}
#endif


// Fast stateless PRNG (PCG32-lite) for Thermal Noise
static inline __device__ float get_thermal_noise(uint32_t neuron_id, uint32_t current_tick, Environment env) {
    uint32_t state = (neuron_id ^ current_tick) * THERMAL_NOISE_MAGIC_A + THERMAL_NOISE_MAGIC_B;
    uint32_t word = ((state >> ((state >> 28u) + 4u)) ^ state) * THERMAL_NOISE_MAGIC_C;
    uint32_t rand_int = (word >> 22u) ^ word;
    // Map to [-1.0, 1.5]
    float normalized = (float)(rand_int % 10000) / 10000.0f; 
    return (normalized * THERMAL_NOISE_SCALE) - 1.0f;
}

// Atomic function: Apoptosis / Death Handler
static inline __device__ void die_gracefully(BaseNeuron* n, uint8_t reason, uint32_t current_tick) {
    if (current_tick < NETWORK_GRACE_PERIOD_TICKS) return;
    
    // Phase 18 Fix: The Dying Breath
    if (current_tick == 5001 && n->is_alive) {
        printf("[Dying Breath] Tick 5001 | Neuron %d | Reason: %d | ATP: %.2f | Calcium: %.2f | V: %.2f\n", 
               n->id, reason, n->energy_ATP, n->calcium_level, n->current_voltage);
    }

    n->is_alive = 0;
    n->death_reason = reason;
    n->current_voltage = 0.0f;
    n->energy_ATP = 0.0f;
    n->calcium_level = 0.0f;
    n->refractory_time_left = 0.0f;
    n->metabolic_wear = 0; // 1.0 packed -> 65535, 0.0 -> 0
}

// Atomic function: metabolism
static inline __device__ void update_metabolism(BaseNeuron* n, Environment env, uint32_t current_tick) {
    if (current_tick == 5005 && n->id == 0) {
        printf("Neuron 0 ATP: %f, Leak: %f\n", n->energy_ATP, BASELINE_LEAK_RATE);
    }

    if (!n->is_alive) {
        n->dead_ticks += 1;
        if (n->dead_ticks >= RESURRECTION_TICK_DELAY) {
            // Phoenix Resurrection
            n->is_alive = 1;
            n->calcium_level = 0.0f;
            n->energy_ATP = n->max_atp; // Phoenix Energy Boost (v0.4.7) - Guaranteed full charge
            n->current_voltage = n->resting_voltage;
            n->dead_ticks = 0;
            n->death_reason = 0;
        }
        return;
    }

    n->internal_phase = (n->internal_phase + 1) % 256;

    // Phase 40: Recovery (Asymptotic to max_atp)
    float atp_gap = n->max_atp - n->energy_ATP;
    if (atp_gap > 0.0f) {
        // Recovery slows down as it reaches max_atp, and is dependent on oxygen
        n->energy_ATP += (ATP_RECOVERY_RATE * env.wakefulness_multiplier * env.global_oxygen_level) * (atp_gap / (n->max_atp * 0.5f));
    }
    
    if (n->energy_ATP > n->max_atp) {
        n->energy_ATP = n->max_atp;
    }

    // Base passive drain + severe hypoxic penalty
    float hypoxia_penalty = 0.0f;
    if (env.global_oxygen_level < MIN_OXYGEN_LEVEL) {
        // As oxygen drops below 0.5, penalty scales up dramatically (up to 0.5 ATP per tick)
        hypoxia_penalty = (0.5f - env.global_oxygen_level) * 1.0f; 
    }
    
    // Fast proxy for incoming activity cost (1/10th of spike cost for processing signals)
    // Avoids atomic collisions from thousands of incoming synapses
    float active_processing_cost = (n->spikes_in_last_window * 0.1f);
    
    float passive_drain = BASELINE_LEAK_RATE + hypoxia_penalty + active_processing_cost;
    
    // Phase 14: Metabolic Calibration (Protocol "Evolution")
    if (current_tick < NETWORK_GRACE_PERIOD_TICKS) {
        passive_drain /= 5.0f;
    }
    
    n->energy_ATP -= passive_drain;

    if (n->predictive_ticks > 0) {
        n->predictive_ticks--;
        if (n->predictive_ticks == 0) {
            n->status_flags &= ~FLAG_IS_PREDICTIVE;
        }
    }

    // Glucose processing
    if (env.global_glucose_level > MIN_GLUCOSE_LEVEL && n->energy_ATP < n->max_atp * GLUCOSE_CONVERT_ATP_RATIO) {
        n->energy_ATP += GLUCOSE_CONVERSION_RATE;
    }

    // Death by starvation/hypoxia
    if (n->energy_ATP <= 0.0f) {
        if (env.global_oxygen_level < 0.3f) {
            n->energy_ATP = 0.0f;
            die_gracefully(n, DEATH_ATP_LOW, current_tick);
        } else if (env.global_glucose_level < MIN_GLUCOSE_LEVEL) {
            n->energy_ATP = 0.0f;
            die_gracefully(n, DEATH_GLUCOSE_STARVATION, current_tick);
        } else {
            // Comatose state
            n->energy_ATP = 0.0f;
            n->current_voltage = n->resting_voltage;
        }
    }
}

static inline __device__ void update_calcium(BaseNeuron* n, uint32_t current_tick) {
    if (n->calcium_level > 0.0f) {
        n->calcium_level -= CALCIUM_RECOVERY_RATE; // Slower recovery
        if (n->calcium_level < 0.0f) n->calcium_level = 0.0f;
    }
    if (n->calcium_level >= FATAL_CALCIUM_LEVEL) die_gracefully(n, DEATH_CALCIUM_HIGH, current_tick);
}

static inline __device__ void evaluate_branches(BaseNeuron* n) {
    #pragma unroll
    for (int i = 0; i < 16; i++) {
        if (!(n->active_branches_mask & (1 << i))) continue;
        
        Compartment* b = &n->branches[i];
        if (b->branch_type == 0) continue;

        // Refractory period for branch
        if (b->refractory_timer > 0) {
            b->refractory_timer--;
            b->voltage = pack_voltage(-100.0f);
            continue;
        }

        float v = unpack_voltage(b->voltage);
        float threshold = unpack_voltage(b->nmda_threshold);

        // Branch Signal Decay (Phase 33/34)
        // Decay towards resting voltage (-70mV approx), not 0.
        v = -70.0f + (v - (-70.0f)) * BRANCH_LEAK_COEFF;
        
        // Passive Leak Gap (Small percentage leak every tick)
        // Only leak the DEPOLARIZATION above resting potential
        float passive_leak = fmaxf(0.0f, v - (-70.0f)) * PASSIVE_LEAK_PERCENT;
        n->current_voltage += passive_leak;
        v -= passive_leak;
        
        // NMDA Spike Logic (Non-linear Summation)
        float soma_impact = 0.0f;
        if (v > threshold) {
            b->local_trace = 255;
            
            if (i <= 3) {
                // Proximal Branch (Branches 0-3): Massive feedforward depolarization (can trigger spike)
                soma_impact = v * NMDA_SPIKE_GAIN;
                n->current_voltage += soma_impact;
            } else {
                // Basal/Apical Branch (Branches 4-15): Contextual prediction! No massive somatic spike.
                // Instead, enter Predictive State for 20 ticks.
                n->predictive_ticks = 20;
                n->status_flags |= (1 << 1); // FLAG_IS_PREDICTIVE
#ifndef USE_CUDA
                if (n->id == 2) printf("Distal branch triggered. V before: %.2f\n", n->current_voltage);
#endif

            }
            
            v = -100.0f; // Reset branch after spike
            b->refractory_timer = BRANCH_REFRACTORY_TICKS; // Block for N ticks
        } else {
            // Passive transfer based on type
            // Only transfer depolarization above resting (-70mV)
            float v_above_rest = fmaxf(0.0f, v - (-70.0f));
            if (b->branch_type == 1) { // Basal
                soma_impact = v_above_rest * BASAL_SOMA_GAIN;
            } else if (b->branch_type == 2) { // Trunk
                soma_impact = v_above_rest * TRUNK_MULTIPLIER;
                n->energy_ATP -= TRUNK_PASSIVE_COST_MULTIPLIER; // Trunk actively costs energy
            } else if (b->branch_type == 3) { // Tuft
                soma_impact = v_above_rest * TUFT_MULTIPLIER;
            }
            n->current_voltage += soma_impact;
        }

        b->voltage = pack_voltage(v);
    }
}

static inline __device__ void process_spike(BaseNeuron* n, Environment env, uint32_t current_tick) {
    // Phase 45: Trace decay
    n->spike_trace *= STDP_DECAY;

    if (n->refractory_time_left > 0.0f) {
        n->refractory_time_left -= 1.0f; 
        n->current_voltage = n->resting_voltage - HYPERPOLARIZATION_OFFSET; // Hyperpolarization

        return; 
    }

    // HTM Predictive State: Temporary depolarization offset (Phase 49)
    if (n->predictive_ticks > 0) {
        // Boost voltage to trigger easier
        if (n->current_voltage < n->resting_voltage + PREDICTIVE_VOLTAGE_BOOST) {
            n->current_voltage = n->resting_voltage + PREDICTIVE_VOLTAGE_BOOST;
        }
    }

    // EMERGENCY GUARD 2.0: Explicit ATP check
    if (n->energy_ATP <= 0.0f) {
        if (env.global_oxygen_level < 0.3f) {
            die_gracefully(n, DEATH_ATP_LOW, current_tick);
        } else if (env.global_glucose_level < MIN_GLUCOSE_LEVEL) {
            die_gracefully(n, DEATH_GLUCOSE_STARVATION, current_tick);
        } else {
            n->current_voltage = n->resting_voltage; // Sleep mode
        }
        return;
    }

    // Metabolic Hysteresis (State Sync v0.4.7)
    if (n->energy_ATP < 15.0f) {
        n->death_reason = STATE_METABOLIC_SHUTDOWN;
    }
    
    if (n->death_reason == STATE_METABOLIC_SHUTDOWN) {
        if (n->energy_ATP >= 40.0f) {
            n->death_reason = DEATH_NONE;
        } else {
            n->current_voltage = n->resting_voltage; // Go to sleep
            return; // CANNOT fire while in shutdown
        }
    }

    // Specialized cell behaviors in idle state
    if (n->type == TYPE_PACEMAKER) {
        // Automatically depolarize over time until spike
        n->current_voltage += PACEMAKER_DEPOLARIZATION;
    } else if (n->type == TYPE_PHOTORECEPTOR) {
        // Sensory Input Summation (Phase 50.1)
        // Use individual external_input (0.0 = black, 1.0 = white pixel)
        float target_v = PHOTO_HYPERPOLARIZED + (1.0f - n->external_input) * (PHOTO_DARK_CURRENT - PHOTO_HYPERPOLARIZED);
        float delta_v = (target_v - n->current_voltage) * PHOTO_ACTIVATION_SPEED;
        
        n->current_voltage += delta_v;
        if (n->current_voltage > 150.0f) n->current_voltage = 150.0f;
        if (n->current_voltage < -100.0f) n->current_voltage = -100.0f;
    }

    float energy_ratio = fmaxf(MIN_ATP_RATIO, n->energy_ATP / n->max_atp);
    float dynamic_threshold = n->activation_threshold * (1.0f - ATP_HUNGER_BETA * (1.0f - energy_ratio));

    if (n->current_voltage > dynamic_threshold) {
        if (n->energy_ATP >= ATP_SPIKE_COST) {
            // Check refractory
            if (current_tick - n->last_spike_tick < (uint32_t)n->refractory_time_left) {
                return;
            }

            // Fire spike
            n->current_voltage = ACTION_POTENTIAL_PEAK;
            n->last_spike_tick = current_tick;
            n->refractory_time_left = ABSOLUTE_REFRACTORY_PERIOD;
            n->spike_trace = 1.0f; // Phase 45: Reset trace on spike
            
            // Metabolic Cost
            n->energy_ATP -= ATP_SPIKE_COST;
            n->calcium_level += CALCIUM_SPIKE_GAIN;
            
            // Track spike count for Window
            n->spikes_in_last_window++;
        } else {
            // Tried to spike but no ATP -> exhaustion collapse and penalty
            n->current_voltage = n->resting_voltage;
            // Still subtract the cost to drive ATP into negative (which triggers coma or death)
            n->energy_ATP -= ATP_SPIKE_COST;
        }
    } else {  
        // SPIKE-DRIVEN WEAR (The "Aging" effect)
        float wear_f = (float)n->metabolic_wear / MAX_U16_FLOAT;
        wear_f += AGING_WEAR_INC; 
        if (wear_f > MAX_METABOLIC_WEAR) wear_f = MAX_METABOLIC_WEAR;
        n->metabolic_wear = (uint16_t)(wear_f * MAX_U16_FLOAT);

        if (n->vesicles_pool >= VESICLES_PER_SPIKE) n->vesicles_pool -= VESICLES_PER_SPIKE;
        
        if (n->type == TYPE_NOCICEPTOR) {
            n->activation_threshold += NOCICEPTOR_SENSITIZATION; // Rapid sensitization
            if (n->activation_threshold < NOCICEPTOR_MIN_THRESHOLD) n->activation_threshold = NOCICEPTOR_MIN_THRESHOLD;
        }
    }
}

static inline __device__ void process_synapse(Synapse* s, BaseNeuron* source, BaseNeuron* target, Environment env, float ltp_mult, float ltd_mult, uint32_t current_tick) {
    // Phase 40 Performance: Early exit for dead neural pathways
    if (!source->is_alive || !target->is_alive) {
        s->weight = fmaxf(MIN_SYNAPSE_WEIGHT, fminf(s->weight, MAX_SYNAPSE_WEIGHT));
        return;
    }

    uint8_t is_myelinated = (s->flags & 0x20) != 0;
    uint8_t is_photoreceptor = (s->flags & 0x40) != 0;

    if (is_photoreceptor) {
        if (source->current_voltage >= PHOTO_CONSTANT_LEAK_THRESHOLD) {
            s->neurotransmitter_buffer += CONSTANT_PHOTORECEPTOR_LEAK; // Photoreceptors leak constantly
            s->ticks_until_delivery = is_myelinated ? MYELINATED_DELIVERY_TICKS : source->axonal_delay;
        }
    } else {
        uint32_t src_last_spike = source->last_spike_tick;
        if (src_last_spike == current_tick) {
            // Bulk release on spike
            s->neurotransmitter_buffer += BULK_NEUROTRANSMITTER_RELEASE; 
            s->ticks_until_delivery = is_myelinated ? MYELINATED_DELIVERY_TICKS : source->axonal_delay; // Myelin Bypass (Phase 26)
        }
    }

    // Phase 45: Spike-Timing-Dependent Plasticity (STDP) with Dopamine Modulation
    uint8_t is_inhibitory = (s->flags & 0x10) != 0;
    
    // Trace-based LTP (Potentiation)
    // If target just spiked, apply LTP based on source's existing trace
    if (ltp_mult > 0.0f) {
        if (!is_inhibitory && target->last_spike_tick == current_tick) {
            uint32_t src_last_spike = source->last_spike_tick; // Lazy load
            if (src_last_spike > 0 && src_last_spike < target->last_spike_tick) {
                int branch_id = s->flags & 0x0F;
                float branch_activity = (float)target->branches[branch_id].local_trace / 255.0f;
                float src_trace = source->spike_trace; // Lazy load
                
                s->weight += src_trace * branch_activity * ltp_mult;
            }
        }
    }

    // Trace-based LTD (Depression) applied exactly when neurotransmitters arrive at the synapse (once per spike)
    if (ltd_mult > 0.0f) {
        uint32_t src_last_spike = source->last_spike_tick; // Lazy load
        if (!is_inhibitory && src_last_spike > 0 && (current_tick - src_last_spike) <= ((s->flags & 0x20) != 0 ? MYELINATED_DELIVERY_TICKS : source->axonal_delay) + 2) {
            // To avoid double LTD penalty where target fired and caused LTP, we should only apply LTD if target fired *before* source
            // If target fired recently (LTP territory), target->last_spike_tick will be GREATER than source->last_spike_tick
            // If target fired BEFORE source, target->last_spike_tick will be LESS than source->last_spike_tick (LTD territory)
            if (target->last_spike_tick < src_last_spike) {
                int branch_id = s->flags & 0x0F;
                float branch_activity = (float)target->branches[branch_id].local_trace / 255.0f;
                
                // Cyber Mutant: Synaptic Consolidation (Weight Locking)
                if (s->weight < MAX_SYNAPSE_WEIGHT * 0.9f) {
                    s->weight -= target->spike_trace * branch_activity * ltd_mult;
                } else {
                    // Small LTD for consolidated synapses
                    s->weight -= target->spike_trace * branch_activity * ltd_mult * 0.1f;
                }
            }
        }
    }

    // Homeostatic Scaling of Weights (Phase 23)
    if (target->spikes_in_last_window > SYNAPSE_SCALING_HIGH) {
        s->weight -= SYNAPSE_PENALTY;
    } else if (target->spikes_in_last_window < SYNAPSE_SCALING_LOW && s->weight < SYNAPSE_RECOVERY_MAX_WEIGHT) {
        s->weight += SYNAPSE_RECOVERY;
    }
    
    // AXON DELIVERY & RECEPTOR BINDING (Spec 06 & 08 + Phase 30)
    if (s->ticks_until_delivery >= 0) {
        if (s->ticks_until_delivery > 0) {
            s->ticks_until_delivery -= 1;
        }
        
        if (s->ticks_until_delivery == 0 && s->neurotransmitter_buffer > 0.0f) {
            // Phase 35: Resource Delivery Check
            if (target->energy_ATP < 5.0f) {
                // Synaptic failure! Target lacks ATP to process incoming neurotransmitters.
                // Neurotransmitters are wasted (cleared by astrocytes later)
                s->neurotransmitter_buffer = 0.0f;
                return;
            }

            // Reliability Check (Phase 30/31)
            // We use weight as a proxy for maturity
            if (s->weight < SYNAPSE_RELIABILITY_THRESHOLD) {
                float failure_roll = (float)((current_tick + s->source_id + s->target_id) % RANDOM_MODULO_BASE) / (float)RANDOM_MODULO_BASE;
                if (failure_roll > SYNAPSE_RELIABILITY_THRESHOLD) {
                    s->neurotransmitter_buffer *= SYNAPSE_RELIABILITY_PENALTY; 
                    s->weight = fmaxf(MIN_SYNAPSE_WEIGHT, fminf(s->weight, MAX_SYNAPSE_WEIGHT));
                    return; 
                }
            }

            // 2. Buffer Capacity
            if (s->neurotransmitter_buffer > MAX_VESICLES) s->neurotransmitter_buffer = MAX_VESICLES;
            
            // Apply bounds to weight (Phase 40 Performance)
            s->weight = fmaxf(MIN_SYNAPSE_WEIGHT, fminf(s->weight, MAX_SYNAPSE_WEIGHT));

            // Receptor Saturation check (Phase 24)
            float available_chemical = s->neurotransmitter_buffer * env.dopamine_multiplier;
            float receptor_limit = s->weight;
            
            float effective_signal = (available_chemical < receptor_limit) ? available_chemical : receptor_limit;
            
            s->last_activation_tick = current_tick;
            s->neurotransmitter_buffer -= (effective_signal / env.dopamine_multiplier); 
            
            int phase_diff = abs((int)source->internal_phase - (int)target->internal_phase);
            if (phase_diff > 128) phase_diff = 256 - phase_diff; // Shortest circular distance
            float phase_multiplier = 1.0f - ((float)phase_diff / 128.0f); // 1.0 if perfectly aligned, 0.0 if opposite
            effective_signal *= phase_multiplier;
            
            float voltage_impact = effective_signal * RECEPTOR_SCALE_CONSTANT; 
            
            int branch_id = s->flags & 0x0F;
            bool is_inhib = (s->flags & 0x10) != 0;
            
            if (is_inhib) {
                atomic_add_packed_voltage(&target->branches[branch_id].voltage, -voltage_impact);
            } else {
                atomic_add_packed_voltage(&target->branches[branch_id].voltage, voltage_impact);
            }
        }
    }

    // Limits (Applied globally at the very end of process_synapse to catch LTP, Homeostatic Scaling, and LTD)
    s->weight = fmaxf(MIN_SYNAPSE_WEIGHT, fminf(s->weight, MAX_SYNAPSE_WEIGHT));
}

#ifdef USE_CUDA
// --- WTA BIT-PACKING ---
// Bits 0-31: float max_v (Voltage)
// Bits 32-53: uint32_t winner_id (22 bits, max ~4 million neurons)
// Bits 54-63: uint16_t tick_mask (10 bits, current_tick % 1024)

__device__ inline unsigned long long int pack_wta(float v, uint32_t id, uint16_t tick_mask) {
    uint32_t v_bits = __float_as_uint(v);
    unsigned long long int packed = 0;
    packed |= (unsigned long long int)v_bits;
    packed |= ((unsigned long long int)(id & 0x3FFFFF) << 32);
    packed |= ((unsigned long long int)(tick_mask & 0x3FF) << 54);
    return packed;
}

__device__ inline void unpack_wta(unsigned long long int packed, float* v, uint32_t* id, uint16_t* tick_mask) {
    uint32_t v_bits = (uint32_t)(packed & 0xFFFFFFFF);
    *v = __uint_as_float(v_bits);
    *id = (uint32_t)((packed >> 32) & 0x3FFFFF);
    *tick_mask = (uint16_t)((packed >> 54) & 0x3FF);
}

__device__ unsigned long long int minicolumn_winners[50000]; // Assume max 1000 columns * 50 minicolumns

// Phase 1: Neuron Biology (Metabolism, Calcium) and WTA Vote
__global__ void tick_neurons_prep_and_vote_kernel(BaseNeuron* neurons, int count, Environment env, uint32_t current_tick) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= count) return;

    BaseNeuron* n = &neurons[idx];

    if (!n->is_alive) {
        n->spike_trace = 0.0f;
        n->predictive_ticks = 0;
        n->dead_ticks += 1;
        if (n->dead_ticks >= RESURRECTION_TICK_DELAY) {
            n->is_alive = 1;
            n->calcium_level = 0.0f;
            n->energy_ATP = n->max_atp;
            n->current_voltage = n->resting_voltage;
            n->dead_ticks = 0;
            n->death_reason = 0;
        }
        return;
    }

    update_metabolism(n, env, current_tick);
    update_calcium(n, current_tick);
    evaluate_branches(n);
    
    // Thermal Noise
    n->current_voltage += get_thermal_noise(n->id, current_tick, env);
    
    // WTA Voting Phase
    if (n->type != TYPE_PYRAMIDAL && n->type != TYPE_INTERNEURON) return;

    if (n->current_voltage > n->activation_threshold) {
        int winner_idx = n->column_id * 50 + n->minicolumn_id;
        
        unsigned long long int* addr_as_ull = &minicolumn_winners[winner_idx];
        unsigned long long int assumed, old = *addr_as_ull;
        uint16_t current_tick_mask = current_tick % 1024;
        
        do {
            assumed = old;
            
            float old_v;
            uint32_t old_id;
            uint16_t old_tick;
            unpack_wta(assumed, &old_v, &old_id, &old_tick);
            
            bool is_pred = (n->status_flags & FLAG_IS_PREDICTIVE) != 0;
            float effective_v = is_pred ? n->current_voltage + 1000.0f : n->current_voltage;
            
            unsigned long long int new_val;
            
            if (old_tick != current_tick_mask) {
                new_val = pack_wta(effective_v, n->id, current_tick_mask);
            } else {
                if (effective_v > old_v || (effective_v == old_v && n->id < old_id)) {
                    new_val = pack_wta(effective_v, n->id, current_tick_mask);
                } else {
                    break;
                }
            }
            
            old = atomicCAS(addr_as_ull, assumed, new_val);
        } while (assumed != old);
    }
}

__global__ void wta_apply_and_spike_kernel(BaseNeuron* neurons, int count, Environment env, uint32_t current_tick) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= count) return;
    
    BaseNeuron* n = &neurons[idx];
    if (!n->is_alive) return;

    if (n->type == TYPE_PYRAMIDAL || n->type == TYPE_INTERNEURON) {
        int winner_idx = n->column_id * 50 + n->minicolumn_id;
        unsigned long long int winner_entry = minicolumn_winners[winner_idx];
        
        float w_v;
        uint32_t w_id;
        uint16_t w_tick;
        unpack_wta(winner_entry, &w_v, &w_id, &w_tick);
        
        uint16_t current_tick_mask = current_tick % 1024;
        
        if (w_tick == current_tick_mask) {
            bool am_i_predictive = (n->status_flags & FLAG_IS_PREDICTIVE) != 0;
            // The winner's v is > 500 if they were predictive
            bool winner_was_predictive = w_v > 500.0f;
            
            if (am_i_predictive && !winner_was_predictive) {
                // Surprise! Bursting (No predictive neurons)
                if (n->current_voltage <= n->activation_threshold) {
                    n->current_voltage = n->activation_threshold + 5.0f;
                }
            } else if (n->id != w_id) {
                n->status_flags |= FLAG_IS_INHIBITED;
                n->current_voltage = n->resting_voltage - 50.0f;
                n->refractory_time_left = 5.0f;
            }
        }
    }

    process_spike(n, env, current_tick);
}

// Optimized Kernel 2: Parallel Synapse Processing (Axon delivery & binding)
__global__ void tick_synapses_kernel(Synapse* synapses, int syn_count, BaseNeuron* neurons, Environment env, uint32_t current_tick) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= syn_count) return;
    
    // Cache heavily used environment variables in registers (Phase 28 Optimization)
    float local_dopamine = env.dopamine_multiplier;
    float local_learning_rate = env.stdp_learning_rate;
    
    Synapse* s = &synapses[idx];
    
    // Tombstoned synapses (Soft Delete) are ignored
    if (s->weight < 0.0f) return;
    
#if __CUDA_ARCH__ >= 350
    // Use __ldg() for read-only access to neurons array
    uint8_t src_alive = __ldg((const uint8_t*)&neurons[s->source_id].is_alive);
    uint8_t tgt_alive = __ldg((const uint8_t*)&neurons[s->target_id].is_alive);
#else
    uint8_t src_alive = neurons[s->source_id].is_alive;
    uint8_t tgt_alive = neurons[s->target_id].is_alive;
#endif

    if (!src_alive || !tgt_alive) {
        s->weight = fmaxf(MIN_SYNAPSE_WEIGHT, fminf(s->weight, MAX_SYNAPSE_WEIGHT));
        return;
    }

    // Pass the modified Environment with cached values back into process_synapse
    // (We modify a local copy of env for the struct to avoid changing function signature)
    Environment local_env = env;
    local_env.dopamine_multiplier = local_dopamine;
    local_env.stdp_learning_rate = local_learning_rate;
    
    // Pre-calculate STDP multipliers to save cycles in inner loop
    float dopa_factor = (local_dopamine > 0.0f) ? local_dopamine : 0.0f;
    float ltp_mult = local_learning_rate * STDP_A_PLUS * dopa_factor;
    float ltd_mult = local_learning_rate * STDP_A_MINUS * dopa_factor;

    process_synapse(s, &neurons[s->source_id], &neurons[s->target_id], local_env, ltp_mult, ltd_mult, current_tick);
}

// --- MACRO BIOLOGY SERVICES ---

__global__ void astrocyte_neurons_kernel(BaseNeuron* neurons, int count) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= count) return;
    
    BaseNeuron* n = &neurons[idx];
    if (n->is_alive && n->energy_ATP < ASTROCYTE_CLEAN_THRESHOLD) {
        n->energy_ATP += ASTROCYTE_ATP_SUPPLY;
        if (n->energy_ATP > n->max_atp) {
            n->energy_ATP = n->max_atp;
        }
    }
    n->spikes_in_last_window = 0; // Reset homeostatic counter
}

__global__ void astrocyte_synapses_kernel(Synapse* synapses, int syn_count) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= syn_count) return;
    
    Synapse* s = &synapses[idx];
    if (s->weight > 0.0f && s->neurotransmitter_buffer > ASTROCYTE_CLEAN_THRESHOLD) {
        s->neurotransmitter_buffer -= ASTROCYTE_CLEAN_AMOUNT;
        if (s->neurotransmitter_buffer < 0.0f) {
            s->neurotransmitter_buffer = 0.0f;
        }
    }
}

// Microglia logic moved to Rust (Phase 31 VRAM Diet)
void launch_microglia_service(BaseNeuron* neurons, int count, Synapse* synapses, int syn_count, uint32_t current_tick) {
    // Intentionally empty. Logic handled by Rust CPU to avoid VRAM atomic bottlenecks.
    (void)neurons;
    (void)count;
    (void)synapses;
    (void)syn_count;
    (void)current_tick;
}

#endif

extern "C" {

void launch_tick_neurons(BaseNeuron* neurons, int count, Synapse* synapses, int syn_count, Environment env, uint32_t current_tick) {
    (void)neurons;
    (void)count;
    (void)synapses;
    (void)syn_count;
    (void)env;
    (void)current_tick;
#ifdef USE_CUDA
    int t_per_b = THREADS_PER_BLOCK;
    int n_blocks = (count + t_per_b - 1) / t_per_b;
    
    // Phase 1: Prepare and Vote
    tick_neurons_prep_and_vote_kernel<<<n_blocks, t_per_b>>>(neurons, count, env, current_tick);
    
    // Phase 2: WTA Apply and Spike
    wta_apply_and_spike_kernel<<<n_blocks, t_per_b>>>(neurons, count, env, current_tick);
    
    // Phase 3: Synapses
    int s_blocks = (syn_count + t_per_b - 1) / t_per_b;
    tick_synapses_kernel<<<s_blocks, t_per_b>>>(synapses, syn_count, neurons, env, current_tick);
    
    // SINGLE SYNCHRONIZATION BARRIER FOR THE ENTIRE TICK
    cudaDeviceSynchronize();
#else
    // Removed CPU fallback loop from here. Rust will handle multithreading using Rayon.
    // Use tick_neuron_cpu and tick_synapse_cpu individually instead.
#endif
}

void launch_astrocyte_service(BaseNeuron* neurons, int count, Synapse* synapses, int syn_count, Environment env) {
    (void)env;
#ifdef USE_CUDA
    int t_per_b = THREADS_PER_BLOCK;
    int n_blocks = (count + t_per_b - 1) / t_per_b;
    int s_blocks = (syn_count + t_per_b - 1) / t_per_b;
    
    astrocyte_neurons_kernel<<<n_blocks, t_per_b>>>(neurons, count);
    astrocyte_synapses_kernel<<<s_blocks, t_per_b>>>(synapses, syn_count);
    
    cudaDeviceSynchronize();
#else
    for (int i = 0; i < count; i++) {
        BaseNeuron* n = &neurons[i];
        if (n->is_alive && n->energy_ATP < ASTROCYTE_CLEAN_THRESHOLD) {
            n->energy_ATP += ASTROCYTE_ATP_SUPPLY;
            if (n->energy_ATP > n->max_atp) {
                n->energy_ATP = n->max_atp;
            }
        }
        n->spikes_in_last_window = 0;
    }
    for (int i = 0; i < syn_count; i++) {
        Synapse* s = &synapses[i];
        if (s->weight > 0.0f && s->neurotransmitter_buffer > ASTROCYTE_CLEAN_THRESHOLD) {
            s->neurotransmitter_buffer -= ASTROCYTE_CLEAN_AMOUNT;
            if (s->neurotransmitter_buffer < 0.0f) {
                s->neurotransmitter_buffer = 0.0f;
            }
        }
    }
#endif
}

void launch_microglia_service(BaseNeuron* neurons, int count, Synapse* synapses, int syn_count, uint32_t current_tick) {
#ifdef USE_CUDA
    int t_per_b = THREADS_PER_BLOCK;
    int n_blocks = (count + t_per_b - 1) / t_per_b;
    int s_blocks = (syn_count + t_per_b - 1) / t_per_b;
    
    microglia_clear_counts_kernel<<<n_blocks, t_per_b>>>(count);
    cudaDeviceSynchronize(); // Ensure counts are cleared
    
    microglia_accumulate_counts_kernel<<<s_blocks, t_per_b>>>(synapses, syn_count);
    cudaDeviceSynchronize(); // Ensure counts are accumulated
    
    microglia_prune_kernel<<<s_blocks, t_per_b>>>(synapses, syn_count, neurons, current_tick);
    cudaDeviceSynchronize();
#else
    // CPU Fallback
    std::vector<int> cpu_outgoing_counts(count, 0);
    
    for (int i = 0; i < syn_count; i++) {
        Synapse* s = &synapses[i];
        if (s->weight > 0.0f) {
            cpu_outgoing_counts[s->source_id]++;
        }
    }
    
    for (int i = 0; i < syn_count; i++) {
        Synapse* s = &synapses[i];
        if (s->weight < 0.0f) continue;
        
        BaseNeuron* src = &neurons[s->source_id];
        BaseNeuron* tgt = &neurons[s->target_id];
        
        bool src_valid = src->is_alive || src->dead_ticks > 0;
        bool tgt_valid = tgt->is_alive || tgt->dead_ticks > 0;
        
        if (!src_valid || !tgt_valid) {
            s->weight = -1.0f; // Tombstone
            continue;
        }
        
        if (cpu_outgoing_counts[s->source_id] <= MIN_SYNAPSES_PER_NEURON) {
            continue;
        }
        
        uint32_t time_since_spike = current_tick > s->last_activation_tick ? current_tick - s->last_activation_tick : 0;
        
        if (s->weight < PRUNING_THRESHOLD && time_since_spike > 100) {
            float inactivity_factor = fminf(1.0f, (float)time_since_spike / 1000.0f);
            float p_drop = (PRUNING_THRESHOLD - s->weight) * inactivity_factor;
            
            uint32_t local_seed = s->source_id ^ s->target_id ^ current_tick;
            float rand_val = get_random_float_01(&local_seed);
            
            if (rand_val < p_drop) {
                cpu_outgoing_counts[s->source_id]--;
                s->weight = -1.0f; // Tombstone
            }
        }
    }
#endif
}

#ifndef USE_CUDA
extern "C" {


    
    void tick_neuron_prepare_cpu(BaseNeuron* neuron, Environment env, uint32_t current_tick) {

        if (!neuron->is_alive) {
            neuron->spike_trace = 0.0f;
            neuron->predictive_ticks = 0;
            neuron->dead_ticks += 1;
            if (neuron->dead_ticks >= RESURRECTION_TICK_DELAY) {
                neuron->is_alive = 1;
                neuron->calcium_level = 0.0f;
                neuron->energy_ATP = neuron->max_atp; 
                neuron->current_voltage = neuron->resting_voltage;
                neuron->dead_ticks = 0;
                neuron->death_reason = 0;
            }
            return;
        }

        update_metabolism(neuron, env, current_tick);
        update_calcium(neuron, current_tick);
        evaluate_branches(neuron);
        
        neuron->current_voltage += get_thermal_noise(neuron->id, current_tick, env);
    }

    void tick_neuron_spike_cpu(BaseNeuron* neuron, Environment env, uint32_t current_tick) {
        if (!neuron->is_alive) return;
        if ((neuron->status_flags & (1 << 0)) != 0) return; // FLAG_IS_INHIBITED
        process_spike(neuron, env, current_tick);
    }

    void tick_neuron_prepare_cpu_batch(BaseNeuron* neurons, int count, Environment env, uint32_t current_tick) {
        for (int i = 0; i < count; i++) {
            if (!neurons[i].is_alive) {
                neurons[i].spike_trace = 0.0f;
                neurons[i].predictive_ticks = 0;
                neurons[i].dead_ticks += 1;
                if (neurons[i].dead_ticks >= RESURRECTION_TICK_DELAY) {
                    neurons[i].is_alive = 1;
                    neurons[i].calcium_level = 0.0f;
                    neurons[i].energy_ATP = neurons[i].max_atp; 
                    neurons[i].current_voltage = neurons[i].resting_voltage;
                    neurons[i].dead_ticks = 0;
                    neurons[i].death_reason = 0;
                }
                continue;
            }

            update_metabolism(&neurons[i], env, current_tick);
            update_calcium(&neurons[i], current_tick);
            evaluate_branches(&neurons[i]);
            
            neurons[i].current_voltage += get_thermal_noise(neurons[i].id, current_tick, env);
            
            if (neurons[i].spikes_in_last_window > 0 && current_tick % 1000 == 0) {
                neurons[i].spikes_in_last_window = 0;
            }
        }
    }

    void tick_neuron_spike_cpu_batch(BaseNeuron* neurons, int count, Environment env, uint32_t current_tick) {
        for (int i = 0; i < count; i++) {
            if (!neurons[i].is_alive) continue;
            if ((neurons[i].status_flags & (1 << 0)) != 0) continue; // FLAG_IS_INHIBITED
            process_spike(&neurons[i], env, current_tick);
        }
    }

    void tick_synapse_cpu(Synapse* synapse, BaseNeuron* neurons, Environment env, uint32_t current_tick) {
        if (synapse->weight < 0.0f) return;
        
        float dopa_factor = (env.dopamine_multiplier > 0.0f) ? env.dopamine_multiplier : 0.0f;
        float ltp_mult = env.stdp_learning_rate * STDP_A_PLUS * dopa_factor;
        float ltd_mult = env.stdp_learning_rate * STDP_A_MINUS * dopa_factor;
        
        process_synapse(synapse, &neurons[synapse->source_id], &neurons[synapse->target_id], env, ltp_mult, ltd_mult, current_tick);
    }

    void tick_synapses_cpu_batch(Synapse* synapses, int count, BaseNeuron* neurons, Environment env, uint32_t current_tick) {
        float dopa_factor = (env.dopamine_multiplier > 0.0f) ? env.dopamine_multiplier : 0.0f;
        float ltp_mult = env.stdp_learning_rate * STDP_A_PLUS * dopa_factor;
        float ltd_mult = env.stdp_learning_rate * STDP_A_MINUS * dopa_factor;
        
        for (int i = 0; i < count; i++) {
            Synapse* s = &synapses[i];
            if (s->weight < 0.0f) continue;
            process_synapse(s, &neurons[s->source_id], &neurons[s->target_id], env, ltp_mult, ltd_mult, current_tick);
        }
    }
}
#endif

void init_neuron(BaseNeuron* neuron, uint32_t id, NeuronType type, uint32_t seed) {
    neuron->id = id;
    neuron->type = type;
    neuron->spike_trace = 0.0f;
    neuron->predictive_ticks = 0; 
    neuron->external_input = 0.0f; // Default black pixel
    
    uint32_t local_seed = seed + id;

    // Heterogeneity: Gaussian distributions (Spec 11)
    float base_resting = NEURON_RESTING_VOLTAGE;
    float base_threshold = NEURON_THRESHOLD;
    
    switch(type) {
        case TYPE_PYRAMIDAL:
            base_threshold = PYRAMIDAL_THRESHOLD; // More excitable
            break;
        case TYPE_INTERNEURON:
            base_resting = INTERNEURON_RESTING; // Faster reset
            base_threshold = INTERNEURON_THRESHOLD;
            break;
        case TYPE_PACEMAKER:
            base_resting = PACEMAKER_RESTING;
            base_threshold = PACEMAKER_THRESHOLD; 
            break;
        case TYPE_PHOTORECEPTOR:
            base_resting = PHOTO_HYPERPOLARIZED;
            base_threshold = PHOTO_THRESHOLD; // Doesn't spike
            break;
        case TYPE_PURKINJE:
            base_resting = PURKINJE_RESTING;
            base_threshold = PURKINJE_THRESHOLD;
            break;
        case TYPE_NOCICEPTOR:
            base_resting = NOCICEPTOR_RESTING;
            base_threshold = NOCICEPTOR_THRESHOLD;
            break;
        default: break;
    }

    neuron->resting_voltage = get_gaussian(&local_seed, base_resting, DEFAULT_VOLTAGE_VAR);
    neuron->activation_threshold = get_gaussian(&local_seed, base_threshold, DEFAULT_VOLTAGE_VAR);
    neuron->current_voltage = neuron->resting_voltage;

    // Spec 11: Metabolic Capacity Variation
    neuron->max_atp = get_gaussian(&local_seed, 100.0f, DEFAULT_ATP_VAR);
    neuron->leak_coeff = get_gaussian(&local_seed, INIT_LEAK_COEFF_MEAN, DEFAULT_LEAK_VAR);
    
    // Randomized structural properties
    neuron->spike_trace = 0.0f;
    neuron->predictive_ticks = 0;
    neuron->vesicles_pool = get_gaussian(&local_seed, MAX_VESICLES, DEFAULT_VESICLES_VAR);

    neuron->energy_ATP = neuron->max_atp;
    neuron->current_voltage = neuron->resting_voltage;
    
    neuron->calcium_level = 0.0f;
    neuron->refractory_time_left = 0.0f;
    neuron->last_spike_tick = 0;
    neuron->spikes_in_last_window = 0;
    neuron->metabolic_wear = 0; // 0.0f packed
    neuron->is_alive = 1;
    neuron->death_reason = DEATH_NONE;
    neuron->internal_phase = 0;
    neuron->dead_ticks = 0;
    
    // Axonal delay is randomized slightly to create asynchronous propagation
    neuron->axonal_delay = (uint8_t)(get_gaussian(&local_seed, 2.0f, 1.0f));
    if (neuron->axonal_delay < 1) neuron->axonal_delay = 1; // Minimum 1 tick delay
    
    float x_f = get_gaussian(&local_seed, DEFAULT_COORD_CENTER, DEFAULT_COORD_VAR);
    float y_f = get_gaussian(&local_seed, DEFAULT_COORD_CENTER, DEFAULT_COORD_VAR);
    float z_f = get_gaussian(&local_seed, DEFAULT_COORD_CENTER, DEFAULT_COORD_VAR);
    
    neuron->x = (uint16_t)(x_f * (MAX_U16_FLOAT / MAX_COORD));
    neuron->y = (uint16_t)(y_f * (MAX_U16_FLOAT / MAX_COORD));
    neuron->z = (uint16_t)(z_f * (MAX_U16_FLOAT / MAX_COORD));

    for (int i = 0; i < NUM_BRANCHES; i++) {
        neuron->branches[i].branch_type = 0;
        neuron->branches[i].refractory_timer = 0;
        neuron->branches[i].voltage = pack_voltage(0.0f);
        neuron->branches[i].nmda_threshold = pack_voltage(0.0f);
        neuron->branches[i].local_trace = 0;
        neuron->branches[i]._pad = 0;
    }
}


void synchronize_device() {
#ifdef USE_CUDA
    cudaDeviceSynchronize();
#endif
}

void cleanup_gpu_resources(void* ptr) {
    if (ptr == nullptr) return;
#ifdef USE_CUDA
    cudaDeviceSynchronize();
    cudaFree(ptr);
#else
    // On CPU, we let Rust's Vec manage its own heap memory via its allocator, 
    // but if we explicitly malloc'd something from C/C++, we'd free it here.
    // Since Rust owns the Vec buffer, we don't call free(ptr) here to avoid double-free/segfaults, 
    // UNLESS we allocated it via cudaMallocManaged equivalent on CPU mock.
    // For pure CPU Rust ownership, this is a no-op.
#endif
}

void debug_get_minicolumn_winner(uint16_t column_id, uint8_t minicolumn_id, float* out_v, uint32_t* out_id) {
#ifdef USE_CUDA
    int winner_idx = column_id * 50 + minicolumn_id;
    unsigned long long int packed;
    cudaMemcpyFromSymbol(&packed, minicolumn_winners, sizeof(unsigned long long int), winner_idx * sizeof(unsigned long long int));
    
    float v;
    uint32_t id;
    uint16_t tick_mask;
    
    uint32_t v_bits = (uint32_t)(packed & 0xFFFFFFFF);
    float tmp_v;
    memcpy(&tmp_v, &v_bits, sizeof(float));
    *out_v = tmp_v;
    
    *out_id = (uint32_t)((packed >> 32) & 0x3FFFFF);
#else
    // CPU fallback is not supported for this global symbol logic right now
    (void)column_id;
    (void)minicolumn_id;
    *out_v = 0.0f;
    *out_id = 0;
#endif
}

} // extern "C"
