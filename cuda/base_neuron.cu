// cuda/base_neuron.cu

#include "base_neuron.h"
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
    uint16_t assumed, old = *address;
    do {
        assumed = old;
        float current_v = unpack_voltage(assumed);
        current_v += voltage_impact;
        uint16_t new_val = pack_voltage(current_v);
        old = atomicCAS(address, assumed, new_val);
    } while (assumed != old);
}
#else
#include <atomic>
static inline void atomic_add_packed_voltage(uint16_t* address, float voltage_impact) {
    std::atomic<uint16_t>* atomic_addr = reinterpret_cast<std::atomic<uint16_t>*>(address);
    uint16_t old = atomic_addr->load(std::memory_order_relaxed);
    while (true) {
        float current_v = unpack_voltage(old);
        current_v += voltage_impact;
        uint16_t new_val = pack_voltage(current_v);
        if (atomic_addr->compare_exchange_weak(old, new_val, std::memory_order_relaxed)) {
            break;
        }
    }
}
#endif


// Fast stateless PRNG (PCG32-lite) for Thermal Noise
static inline __device__ float get_thermal_noise(uint32_t neuron_id, uint32_t current_tick) {
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

    if (!n->is_alive) {
        n->dead_ticks += 1;
        if (n->dead_ticks >= RESURRECTION_TICK_DELAY) {
            // Phoenix Resurrection
            n->is_alive = 1;
            n->calcium_level = 0.0f;
            n->energy_ATP = n->max_atp;
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
        // Recovery slows down as it reaches max_atp
        n->energy_ATP += (ATP_RECOVERY_RATE * env.wakefulness_multiplier) * (atp_gap / (n->max_atp * 0.5f));
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
    float passive_drain = BASELINE_LEAK_RATE + hypoxia_penalty;
    
    // Phase 14: Metabolic Calibration (Protocol "Evolution")
    if (current_tick < NETWORK_GRACE_PERIOD_TICKS) {
        passive_drain /= 5.0f;
    }
    
    n->energy_ATP -= passive_drain;

    // Glucose processing
    if (env.global_glucose_level > MIN_GLUCOSE_LEVEL && n->energy_ATP < n->max_atp * GLUCOSE_CONVERT_ATP_RATIO) {
        n->energy_ATP += GLUCOSE_CONVERSION_RATE;
    }

    // Death by starvation/hypoxia
    if (n->energy_ATP <= 0.0f) {
        n->energy_ATP = 0.0f;
        if (env.global_oxygen_level < MIN_OXYGEN_LEVEL) {
            die_gracefully(n, DEATH_ATP_LOW, current_tick);
        } else if (env.global_glucose_level < MIN_GLUCOSE_LEVEL) {
            die_gracefully(n, DEATH_GLUCOSE_STARVATION, current_tick);
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
    for (int i = 0; i < NUM_BRANCHES; i++) {
        Compartment* b = &n->branches[i];
        if (b->branch_type == 0) continue;

        // Refractory period for branch
        if (b->refractory_timer > 0) {
            b->refractory_timer--;
            b->voltage = pack_voltage(0.0f);
            continue;
        }

        float v = unpack_voltage(b->voltage);
        float threshold = unpack_voltage(b->nmda_threshold);

        // Branch Signal Decay (Phase 33/34)
        v *= BRANCH_LEAK_COEFF;
        
        // Passive Leak Gap (Small percentage leak every tick)
        float passive_leak = v * PASSIVE_LEAK_PERCENT;
        n->current_voltage += passive_leak;
        v -= passive_leak;
        
        // NMDA Spike Logic (Non-linear Summation)
        float soma_impact = 0.0f;
        if (v > threshold) {
            b->local_trace = 255;
            
            if (i == 0) {
                // Proximal Branch (Branch 0): Massive feedforward depolarization (can trigger spike)
                soma_impact = v * NMDA_SPIKE_GAIN;
            } else {
                // Basal/Apical Branch (Branches 1-7): Contextual prediction! No massive somatic spike.
                // Instead, enter Predictive State for 20 ticks.
                n->predictive_ticks = 20;
            }
            
            v = 0.0f; // Reset branch after spike
            b->refractory_timer = BRANCH_REFRACTORY_TICKS; // Block for N ticks
        } else {
            // Passive transfer based on type
            if (b->branch_type == 1) soma_impact = v * BASAL_SOMA_GAIN; // Basal
            else if (b->branch_type == 2) { // Apical Trunk (Boosted)
                if (n->energy_ATP >= TRUNK_ATP_COST * TRUNK_PASSIVE_COST_MULTIPLIER) {
                    soma_impact = v * TRUNK_MULTIPLIER;
                    n->energy_ATP -= TRUNK_ATP_COST * TRUNK_PASSIVE_COST_MULTIPLIER;
                } else {
                    soma_impact = v * BASAL_SOMA_GAIN; // Downgrade to passive if starved
                }
            } else if (b->branch_type == 3) { // Apical Tuft (Distant)
                soma_impact = v * TUFT_MULTIPLIER;
            }
        }
        
        // Attenuation for somatic impact
        if (i == 0) {
            float attenuation = 0.8f; // Proximal is strong
            n->current_voltage += soma_impact * attenuation;
        } else {
            // Non-proximal branches apply normal passive transfer (which is already scaled by BASAL_SOMA_GAIN etc)
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
        if (n->predictive_ticks > 0) n->predictive_ticks--;
        return; 
    }

    // HTM Predictive State: Temporary depolarization offset (Phase 49)
    if (n->predictive_ticks > 0) {
        // Boost voltage to trigger easier
        n->current_voltage += PREDICTIVE_VOLTAGE_BOOST;
        n->predictive_ticks--;
    }

    // EMERGENCY GUARD 2.0: Explicit ATP check
    if (n->energy_ATP <= 0.0f) {
        if (env.global_oxygen_level < MIN_OXYGEN_LEVEL) {
            die_gracefully(n, DEATH_ATP_LOW, current_tick);
        } else if (env.global_glucose_level < MIN_GLUCOSE_LEVEL) {
            die_gracefully(n, DEATH_GLUCOSE_STARVATION, current_tick);
        } else {
            n->current_voltage = n->resting_voltage; // Sleep mode
        }
        return;
    }

    // Phase 18 Fix: "Metabolic Coma" (Safety Valve)
    if (n->energy_ATP < n->max_atp * 0.10f) {
        n->current_voltage = n->resting_voltage;
        return;
    }

    // Specialized cell behaviors in idle state
    if (n->type == TYPE_PACEMAKER) {
        // Automatically depolarize over time until spike
        n->current_voltage += PACEMAKER_DEPOLARIZATION;
    } else if (n->type == TYPE_PHOTORECEPTOR) {
        // Dark current: depolarize to -40mV in dark, hyperpolarize to -65mV in light
        float target_v = PHOTO_HYPERPOLARIZED + (1.0f - env.light_level) * (PHOTO_DARK_CURRENT - PHOTO_HYPERPOLARIZED);
        n->current_voltage += (target_v - n->current_voltage) * PHOTO_ACTIVATION_SPEED;
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

static inline __device__ void process_synapse(Synapse* s, BaseNeuron* source, BaseNeuron* target, Environment env, uint32_t current_tick) {
    // Phase 40 Performance: Early exit for dead neural pathways
    if (!source->is_alive || !target->is_alive) {
        s->weight = fmaxf(MIN_SYNAPSE_WEIGHT, fminf(s->weight, MAX_SYNAPSE_WEIGHT));
        return;
    }

    if (source->type == TYPE_PHOTORECEPTOR) {
        if (source->current_voltage >= PHOTO_CONSTANT_LEAK_THRESHOLD) {
            s->neurotransmitter_buffer += CONSTANT_PHOTORECEPTOR_LEAK; // Photoreceptors leak constantly
            uint8_t is_myelinated = (s->flags & 0x10) != 0;
            s->ticks_until_delivery = is_myelinated ? MYELINATED_DELIVERY_TICKS : UNMYELINATED_DELIVERY_TICKS;
        }
    } else {
        if (source->last_spike_tick == current_tick) {
            // Bulk release on spike
            s->neurotransmitter_buffer += BULK_NEUROTRANSMITTER_RELEASE; 
            uint8_t is_myelinated = (s->flags & 0x10) != 0;
            s->ticks_until_delivery = is_myelinated ? MYELINATED_DELIVERY_TICKS : UNMYELINATED_DELIVERY_TICKS; // Myelin Bypass (Phase 26)
        }
    }

    // Phase 45: Spike-Timing-Dependent Plasticity (STDP) with Dopamine Modulation
    uint8_t is_inhibitory = (s->flags & 0x08) != 0;
    
    // Trace-based LTP (Potentiation)
    // If target just spiked, apply LTP based on source's existing trace
    if (!is_inhibitory && target->last_spike_tick == current_tick && source->last_spike_tick > 0 && source->last_spike_tick < target->last_spike_tick) {
        int branch_id = s->flags & 0x07;
        float branch_activity = (float)target->branches[branch_id].local_trace / 255.0f;
        float delta_w = source->spike_trace * env.stdp_learning_rate * STDP_A_PLUS;
        s->weight += delta_w * env.dopamine_multiplier * branch_activity;
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
            // Trace-based LTD (Depression) applied exactly when neurotransmitters arrive at the synapse (once per spike)
            uint32_t expected_delivery_delay = ((s->flags & 0x10) != 0) ? MYELINATED_DELIVERY_TICKS : UNMYELINATED_DELIVERY_TICKS;
                                                            if (!is_inhibitory && source->last_spike_tick > 0 && (current_tick - source->last_spike_tick) <= expected_delivery_delay + 2) {
                // To avoid double LTD penalty where target fired and caused LTP, we should only apply LTD if target fired *before* source
                // If target fired recently (LTP territory), target->last_spike_tick will be GREATER than source->last_spike_tick
                // In acausal firing (LTD territory), target fired, then source fired, so target->last_spike_tick <= source->last_spike_tick
                if (target->last_spike_tick > 0 && target->last_spike_tick <= source->last_spike_tick) {
                    float delta_w = target->spike_trace * env.stdp_learning_rate * STDP_A_MINUS;
                    s->weight -= delta_w * env.dopamine_multiplier;
                }
            } 

            // Reliability Check (Phase 30/31)
                        // We use weight as a proxy for maturity
            if (s->weight < SYNAPSE_RELIABILITY_THRESHOLD) {
                float failure_roll = (float)((current_tick + s->source_id + s->target_id) % RANDOM_MODULO_BASE) / (float)RANDOM_MODULO_BASE;
                if (failure_roll > SYNAPSE_RELIABILITY_THRESHOLD) {
                    s->neurotransmitter_buffer *= SYNAPSE_RELIABILITY_PENALTY; 
                    return; 
                }
            }

            // 2. Buffer Capacity
            if (s->neurotransmitter_buffer > MAX_VESICLES) s->neurotransmitter_buffer = MAX_VESICLES;

            // RECEPTOR SATURATION (Spec 08)
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
            
            int branch_id = s->flags & 0x07;
            bool is_inhib = (s->flags & 0x08) != 0;
            
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
// Optimized Kernel 1: Only Neuron Biology (Metabolism, Calcium, Spiking)
__global__ void tick_neurons_kernel(BaseNeuron* neurons, int count, Environment env, uint32_t current_tick) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= count) return;
    
    // Trace Cleanup: Ensure spike_trace is properly decayed or zeroed

    if (!neurons[idx].is_alive) {
        neurons[idx].spike_trace = 0.0f;
        neurons[idx].dead_ticks += 1;
        if (neurons[idx].dead_ticks >= RESURRECTION_TICK_DELAY) {
            // Phoenix Resurrection
            neurons[idx].is_alive = true;
            neurons[idx].calcium_level = 0.0f;
            neurons[idx].energy_ATP = neurons[idx].max_atp;
            neurons[idx].current_voltage = neurons[idx].resting_voltage;
            neurons[idx].dead_ticks = 0;
            neurons[idx].death_reason = 0;
        }
        return;
    }

    

    update_metabolism(&neurons[idx], env, current_tick);
    update_calcium(&neurons[idx], current_tick);
    evaluate_branches(&neurons[idx]);
    process_spike(&neurons[idx], env, current_tick);
    
    // Thermal Noise (Phase 48)
    neurons[idx].current_voltage += get_thermal_noise(neurons[idx].id, current_tick);

}

// Optimized Kernel 2: Parallel Synapse Processing (Axon delivery & binding)
__global__ void tick_synapses_kernel(Synapse* synapses, int syn_count, BaseNeuron* neurons, Environment env, uint32_t current_tick) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= syn_count) return;
    
    Synapse* s = &synapses[idx];
    
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

    process_synapse(s, &neurons[s->source_id], &neurons[s->target_id], env, current_tick);
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
    tick_neurons_kernel<<<n_blocks, t_per_b>>>(neurons, count, env, current_tick);
    cudaDeviceSynchronize(); 
    
    int s_blocks = (syn_count + t_per_b - 1) / t_per_b;
    tick_synapses_kernel<<<s_blocks, t_per_b>>>(synapses, syn_count, neurons, env, current_tick);
    cudaDeviceSynchronize();
#else
    // Removed CPU fallback loop from here. Rust will handle multithreading using Rayon.
    // Use tick_neuron_cpu and tick_synapse_cpu individually instead.
#endif
}

#ifndef USE_CUDA
extern "C" {

    void tick_neuron_cpu(BaseNeuron* neuron, Environment env, uint32_t current_tick) {
        if (!neuron->is_alive) {
            neuron->spike_trace = 0.0f;
            neuron->predictive_ticks = 0;
            neuron->dead_ticks += 1;
            if (neuron->dead_ticks >= RESURRECTION_TICK_DELAY) {
                // Phoenix Resurrection
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
        process_spike(neuron, env, current_tick);
        
        // Thermal Noise (Phase 48)
        neuron->current_voltage += get_thermal_noise(neuron->id, current_tick);

    }

    void tick_synapse_cpu(Synapse* synapse, BaseNeuron* neurons, Environment env, uint32_t current_tick) {
        process_synapse(synapse, &neurons[synapse->source_id], &neurons[synapse->target_id], env, current_tick);
    }
}
#endif

void init_neuron(BaseNeuron* neuron, uint32_t id, NeuronType type, uint32_t seed) {
    neuron->id = id;
    neuron->type = type;
    neuron->spike_trace = 0.0f;
    neuron->predictive_ticks = 0; 
    
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
} // extern "C"
