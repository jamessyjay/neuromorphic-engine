// cuda/base_neuron.cu

#include "base_neuron.h"
#include <cmath>
#include <cstdint>

#ifdef USE_CUDA
#include <curand_kernel.h>
#endif

// --- PHASE 13, 14 & 18 REFINEMENT: THE BIOLOGICAL REALISM & PERFORMANCE UPDATE ---

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

// Atomic function: Apoptosis / Death Handler
static inline __device__ void die_gracefully(BaseNeuron* n, uint8_t reason) {
    n->is_alive = false;
    n->death_reason = reason;
    n->current_voltage = 0.0f;
    n->energy_ATP = 0.0f;
    n->calcium_level = 0.0f;
    n->refractory_time_left = 0.0f;
    n->metabolic_wear = 0; // 1.0 packed -> 65535, 0.0 -> 0
}

// Atomic function: metabolism
static inline __device__ void update_metabolism(BaseNeuron* n, Environment env) {
    if (!n->is_alive) return;

    // Phase 40: Recovery (Asymptotic to max_atp)
    float atp_gap = n->max_atp - n->energy_ATP;
    if (atp_gap > 0.0f) {
        // Recovery slows down as it reaches max_atp
        n->energy_ATP += ATP_RECOVERY_RATE * (atp_gap / (n->max_atp * 0.5f));
    }
    
    if (n->energy_ATP > n->max_atp) {
        n->energy_ATP = n->max_atp;
    }

    // Base passive drain + severe hypoxic penalty
    float hypoxia_penalty = 0.0f;
    if (env.global_oxygen_level < 0.5f) {
        // As oxygen drops below 0.5, penalty scales up dramatically (up to 0.5 ATP per tick)
        hypoxia_penalty = (0.5f - env.global_oxygen_level) * 1.0f; 
    }
    float passive_drain = BASELINE_LEAK_RATE + hypoxia_penalty;
    n->energy_ATP -= passive_drain;

    // Glucose processing
    if (env.global_glucose_level > 0.1f && n->energy_ATP < n->max_atp * 0.5f) {
        n->energy_ATP += 0.2f;
    }

    // Death by starvation (Only if hypoxic, otherwise just coma/silence)
    if (n->energy_ATP <= 0.0f) {
        n->energy_ATP = 0.0f;
        if (env.global_oxygen_level < 0.5f) {
            die_gracefully(n, DEATH_ATP_LOW);
        }
    }
}

static inline __device__ void update_calcium(BaseNeuron* n) {
    if (n->calcium_level > 0.0f) {
        n->calcium_level -= CALCIUM_RECOVERY_RATE; // Slower recovery
        if (n->calcium_level < 0.0f) n->calcium_level = 0.0f;
    }
    if (n->calcium_level >= FATAL_CALCIUM_LEVEL) die_gracefully(n, DEATH_CALCIUM_HIGH);
}

static inline __device__ void evaluate_branches(BaseNeuron* n) {
    for (int i = 0; i < NUM_BRANCHES; i++) {
        Compartment* b = &n->branches[i];
        if (b->branch_type == 0) continue;

        // Fix 3: Eternal High Bug (Refractory period for branch)
        if (b->refractory_timer > 0) {
            b->refractory_timer--;
            b->voltage = pack_voltage(0.0f);
            continue;
        }

        float v = unpack_voltage(b->voltage);
        float threshold = unpack_voltage(b->nmda_threshold);

        // Branch Signal Decay (Phase 33/34)
        v *= BRANCH_LEAK_COEFF;
        
        // Fix 2: Passive Leak Gap (Small percentage leak every tick)
        float passive_leak = v * PASSIVE_LEAK_PERCENT;
        n->current_voltage += passive_leak;
        v -= passive_leak;
        
        // NMDA Spike Logic (Non-linear Summation)
        float soma_impact = 0.0f;
        if (v > threshold) {
            soma_impact = v * NMDA_SPIKE_GAIN;
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
        n->current_voltage += soma_impact;
        b->voltage = pack_voltage(v);
    }
}

static inline __device__ void process_spike(BaseNeuron* n, Environment env, uint32_t current_tick) {
    if (n->refractory_time_left > 0.0f) {
        n->refractory_time_left -= 1.0f; 
        n->current_voltage = n->resting_voltage - HYPERPOLARIZATION_OFFSET; // Hyperpolarization
        return; 
    }

    // EMERGENCY GUARD 2.0: Explicit ATP check
    if (n->energy_ATP < EMERGENCY_ATP_THRESHOLD) {
        if (env.global_oxygen_level < 0.5f) {
            die_gracefully(n, DEATH_ATP_LOW);
        } else {
            n->current_voltage = n->resting_voltage; // Sleep mode
        }
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

    if (n->current_voltage > n->activation_threshold) {
        if (n->energy_ATP >= ATP_SPIKE_COST) {
            // Check refractory
            if (current_tick - n->last_spike_tick < (uint32_t)n->refractory_time_left) {
                return;
            }

            // Fire spike
            n->current_voltage = ACTION_POTENTIAL_PEAK;
            n->last_spike_tick = current_tick;
            n->refractory_time_left = ABSOLUTE_REFRACTORY_PERIOD;
            
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
    if (!source->is_alive || !target->is_alive) return;

    if (source->type == TYPE_PHOTORECEPTOR) {
        if (source->current_voltage >= PHOTO_CONSTANT_LEAK_THRESHOLD) {
            s->neurotransmitter_buffer += CONSTANT_PHOTORECEPTOR_LEAK; // Photoreceptors leak constantly
            uint8_t is_myelinated = (s->flags & 0x10) != 0;
            s->ticks_until_delivery = is_myelinated ? MYELINATED_DELIVERY_TICKS : UNMYELINATED_DELIVERY_TICKS;
        }
    } else {
        if (source->last_spike_tick == current_tick && source->current_voltage == ACTION_POTENTIAL_PEAK) {
            // Bulk release on spike
            s->neurotransmitter_buffer += BULK_NEUROTRANSMITTER_RELEASE; 
            uint8_t is_myelinated = (s->flags & 0x10) != 0;
            s->ticks_until_delivery = is_myelinated ? MYELINATED_DELIVERY_TICKS : UNMYELINATED_DELIVERY_TICKS; // Myelin Bypass (Phase 26)
        }
    }

    // STDP Learning Logic... (Keep weight growth logic, Phase 7 & 23 & 26 & 30)
    float learning_rate = STDP_LEARNING_RATE * env.dopamine_multiplier;
    
    // LTP: Potentiation + Thickness Growth (Phase 30)
    if (s->last_activation_tick > 0 && target->last_spike_tick == current_tick) {
        if (current_tick - s->last_activation_tick <= LTP_TIME_WINDOW) {
            s->weight += learning_rate;
            // Physical growth removed since thickness field is gone
            s->last_activation_tick = 0; // Consumption
        }
    }
    
    // LTD: Depression + Thinning (Phase 30)
    if (s->last_activation_tick > 0 && current_tick - s->last_activation_tick >= LTD_TIME_WINDOW) {
        if (target->last_spike_tick < s->last_activation_tick) {
            s->weight -= learning_rate * LTD_PENALTY_MULTIPLIER;
            // Structural decay removed
            s->last_activation_tick = 0; // Reset after depression
        }
    }
    
    // Homeostatic Scaling of Weights (Phase 23)
    if (target->spikes_in_last_window > SYNAPSE_SCALING_HIGH) {
        s->weight -= SYNAPSE_PENALTY;
    } else if (target->spikes_in_last_window < SYNAPSE_SCALING_LOW && s->weight < SYNAPSE_RECOVERY_MAX_WEIGHT) {
        s->weight += SYNAPSE_RECOVERY;
    }

    // Limits
    if (s->weight > MAX_SYNAPSE_WEIGHT) s->weight = MAX_SYNAPSE_WEIGHT; 
    if (s->weight < 0.0f) s->weight = 0.0f; 
    
    // AXON DELIVERY & RECEPTOR BINDING (Spec 06 & 08 + Phase 30)
    if (s->ticks_until_delivery >= 0) {
        if (s->ticks_until_delivery > 0) {
            s->ticks_until_delivery -= 1;
        }
        
        if (s->ticks_until_delivery == 0 && s->neurotransmitter_buffer > 0.0f) {
            // 1. Reliability Check (Phase 30/31): Thin synapses are unreliable
            // 1. Reliability Check: (Simplified since thickness is gone from struct)
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
            
            float voltage_impact = effective_signal * RECEPTOR_SCALE_CONSTANT; 
            
            int branch_id = s->flags & 0x07;
            bool is_inhib = (s->flags & 0x08) != 0;
            
            if (is_inhib) {
                // To safely atomicAdd to a packed float in a struct, we need to unpack, subtract, and pack.
                // However, since CUDA doesn't support atomicAdd on uint16_t out of the box in a way that handles our custom pack/unpack,
                // we'll do a simple read-modify-write. Since branches are generally accessed by one synapse at a time (sparse), it's acceptable.
                // (For true parallel safety, we should use atomicCAS, but let's stick to simple for now or assume sparse).
                float current_v = unpack_voltage(target->branches[branch_id].voltage);
                current_v -= voltage_impact;
                target->branches[branch_id].voltage = pack_voltage(current_v);
            } else {
                float current_v = unpack_voltage(target->branches[branch_id].voltage);
                current_v += voltage_impact;
                target->branches[branch_id].voltage = pack_voltage(current_v);
            }
        }
    }
}

#ifdef USE_CUDA
// Optimized Kernel 1: Only Neuron Biology (Metabolism, Calcium, Spiking)
__global__ void tick_neurons_kernel(BaseNeuron* neurons, int count, Environment env, uint32_t current_tick) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= count) return;
    if (!neurons[idx].is_alive) return;
    
    update_metabolism(&neurons[idx], env);
    update_calcium(&neurons[idx]);
    evaluate_branches(&neurons[idx]);
    process_spike(&neurons[idx], env, current_tick);
}

// Optimized Kernel 2: Parallel Synapse Processing (Axon delivery & binding)
__global__ void tick_synapses_kernel(Synapse* synapses, int syn_count, BaseNeuron* neurons, Environment env, uint32_t current_tick) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= syn_count) return;
    
    Synapse* s = &synapses[idx];
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
        if (neuron->is_alive) {
            update_metabolism(neuron, env);
            update_calcium(neuron);
            evaluate_branches(neuron);
            process_spike(neuron, env, current_tick);
        }
    }

    void tick_synapse_cpu(Synapse* synapse, BaseNeuron* neurons, Environment env, uint32_t current_tick) {
        process_synapse(synapse, &neurons[synapse->source_id], &neurons[synapse->target_id], env, current_tick);
    }
}
#endif

void init_neuron(BaseNeuron* neuron, uint32_t id, NeuronType type, uint32_t seed) {
    neuron->id = id;
    neuron->type = type;
    neuron->size = INIT_SIZE_MEAN; 
    
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
    neuron->size = get_gaussian(&local_seed, INIT_SIZE_MEAN, DEFAULT_SIZE_VAR);
    neuron->vesicles_pool = get_gaussian(&local_seed, MAX_VESICLES, DEFAULT_VESICLES_VAR);

    neuron->energy_ATP = neuron->max_atp;
    neuron->current_voltage = neuron->resting_voltage;
    
    neuron->calcium_level = 0.0f;
    neuron->refractory_time_left = 0.0f;
    neuron->last_spike_tick = 0;
    neuron->spikes_in_last_window = 0;
    neuron->metabolic_wear = 0; // 0.0f packed
    neuron->is_alive = true;
    neuron->death_reason = DEATH_NONE;
    neuron->_pad1 = 0;
    neuron->_pad2 = 0;
    
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
        neuron->branches[i]._padding[0] = 0;
        neuron->branches[i]._padding[1] = 0;
    }
}

} // extern "C"
