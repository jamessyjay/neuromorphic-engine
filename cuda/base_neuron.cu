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

// Atomic function: Apoptosis / Death Handler
static inline __device__ void die_gracefully(BaseNeuron* n) {
    n->is_alive = false;
    n->current_voltage = 0.0f; 
    n->energy_ATP = 0.0f;
    n->calcium_level = 0.0f;
    n->refractory_time_left = 0.0f;
    n->metabolic_wear = 1.0f; // Mitochondria dead
}

// Atomic function: metabolism
static inline __device__ void update_metabolism(BaseNeuron* n, Environment env, uint32_t current_tick) {
    if (n->energy_ATP <= 1.0f) {
        die_gracefully(n);
        return;
    }

    // 1. ATP Generation (mitochondria) - Asymptotic Recovery (Phase 28)
    float baseline_generation = 1.0f * (1.0f - n->metabolic_wear);
    float glucose_factor = env.global_glucose_level;
    float oxygen_factor = env.global_oxygen_level;
    
    // Recovery slows down as capacity is reached (Asymptotic)
    float atp_gap = n->max_atp - n->energy_ATP;
    n->energy_ATP += (atp_gap / n->max_atp) * baseline_generation * oxygen_factor * glucose_factor;
    if (n->energy_ATP > n->max_atp) n->energy_ATP = n->max_atp;

    // 2. Vesicle Recycling (Phase 26/28) - Spec 02
    // Recovery slows down as pool fills
    float v_gap = 100.0f - n->vesicles_pool;
    n->vesicles_pool += (v_gap / 100.0f) * 0.5f * (n->energy_ATP / 100.0f);
    if (n->vesicles_pool > 100.0f) n->vesicles_pool = 100.0f;

    // 3. Homeostatic Plasticity & Threshold Recovery (Phase 13 & 26)
    if (n->spikes_in_last_window > 20) {
        n->activation_threshold += 0.5f; // Raise threshold (Negative Feedback)
    } else if (n->spikes_in_last_window < 2) {
        // Recovery: Move back to resting threshold (Spec 11)
        float base_threshold = -55.0f; // Standard Pyramidal
        if (n->type == TYPE_PHOTORECEPTOR) base_threshold = 0.0f;
        
        if (n->activation_threshold > base_threshold) {
             n->activation_threshold -= 0.1f;
        } else if (n->activation_threshold < base_threshold) {
             n->activation_threshold += 0.1f;
        }
    }
    n->spikes_in_last_window = 0; 

    // BASELINE LEAKAGE (Spec 11)
    float gradient_maintenance = 0.05f * n->leak_coeff;
    if (n->energy_ATP < 30.0f) gradient_maintenance *= 1.5f; 
    n->energy_ATP -= gradient_maintenance;

    // Step 2.5: EMERGENCY METABOLIC GUARD (Phase 23)
    if (n->energy_ATP < 20.0f) {
        n->activation_threshold = 20.0f; // Force silence
    }
    
    // Step 3: Voltage drift & CHAOS (Spec 11)
    uint32_t chaos_seed = env.seed + n->id + current_tick;
    float noise = get_gaussian(&chaos_seed, 0.0f, 0.1f);
    n->current_voltage += noise;

    if (n->type == TYPE_PACEMAKER) {
        n->current_voltage += 0.7f; 
        n->energy_ATP -= 0.1f * n->leak_coeff;
    } else if (n->type == TYPE_PHOTORECEPTOR) {
        // Photoreceptor logic: 
        // In light -> hyperpolarize (-65mV)
        // In dark -> depolarize (-40mV, dark current)
        float target_v = (env.light_level > 0.5f) ? -65.0f : -40.0f;
        float sens_speed = 0.5f;
        if (n->current_voltage < target_v) n->current_voltage += sens_speed;
        else if (n->current_voltage > target_v) n->current_voltage -= sens_speed;
    } else {
        // Drift towards resting potential
        float drift = 0.2f * n->leak_coeff;
        if (n->current_voltage > n->resting_voltage) {
            n->current_voltage -= drift; 
            if (n->current_voltage < n->resting_voltage) n->current_voltage = n->resting_voltage;
        } else if (n->current_voltage < n->resting_voltage) {
            n->current_voltage += drift;
        }
    }
}

static inline __device__ void update_calcium(BaseNeuron* n) {
    if (n->calcium_level > 0.0f) {
        n->calcium_level -= 0.5f; // Slower recovery
        if (n->calcium_level < 0.0f) n->calcium_level = 0.0f;
    }
    if (n->calcium_level >= FATAL_CALCIUM_LEVEL) die_gracefully(n);
}

static inline __device__ void process_spike(BaseNeuron* n, uint32_t current_tick) {
    if (n->refractory_time_left > 0.0f) {
        n->refractory_time_left -= 1.0f; 
        n->current_voltage = n->resting_voltage - 10.0f; // Hyperpolarization
        return; 
    }

    // EMERGENCY GUARD 2.0: Explicit ATP check
    if (n->energy_ATP < 20.0f) return;

    if (n->current_voltage >= n->activation_threshold) {
        if (n->type == TYPE_PHOTORECEPTOR) return; // Sensors don't spike
        
        // BOOM: SPIKE!
        n->current_voltage = SPIKE_VOLTAGE;
        n->refractory_time_left = (REFRACTORY_PERIOD_ABSOLUTE_MS + REFRACTORY_PERIOD_RELATIVE_MS) * 1.5f;
        n->last_spike_tick = current_tick;
        
        // Metabolic Cost (Spec 11)
        n->energy_ATP -= ATP_SPIKE_COST;
        n->calcium_level += CALCIUM_SPIKE_GAIN;
        n->spikes_in_last_window++;
        
        // SPIKE-DRIVEN WEAR (The "Aging" effect)
        n->metabolic_wear += 0.0005f; 
        if (n->metabolic_wear > 0.98f) n->metabolic_wear = 0.98f;

        if (n->vesicles_pool >= 5.0f) n->vesicles_pool -= 5.0f;
        
        if (n->type == TYPE_NOCICEPTOR) {
            n->activation_threshold -= 2.0f; // Rapid sensitization
            if (n->activation_threshold < -60.0f) n->activation_threshold = -60.0f;
        }
    }
}

static inline __device__ void process_synapse(Synapse* s, BaseNeuron* source, BaseNeuron* target, Environment env, uint32_t current_tick) {
    if (source->is_alive) {
        if (source->type == TYPE_PHOTORECEPTOR) {
            if (source->current_voltage >= -41.0f) {
                s->neurotransmitter_buffer += 2.0f; // Photoreceptors leak constantly
                s->ticks_until_delivery = s->is_myelinated ? 0 : 2;
            }
        } else {
            if (source->last_spike_tick == current_tick && source->current_voltage == SPIKE_VOLTAGE) {
                // Bulk release on spike
                s->neurotransmitter_buffer += 25.0f; 
                s->ticks_until_delivery = s->is_myelinated ? 0 : 2; // Myelin Bypass (Phase 26)
            }
        }
    }

    // STDP Learning Logic (Phase 7 & 23 & 26)
    float learning_rate = 0.1f * env.dopamine_multiplier;
    
    // LTP: Potentiation (Source then Target)
    if (s->last_activation_tick > 0 && target->last_spike_tick == current_tick) {
        if (current_tick - s->last_activation_tick <= 20) {
            s->weight += learning_rate;
            s->last_activation_tick = 0; // Consumption
        }
    }
    
    // LTD: Depression (Forget non-effective pulses) - Phase 26
    if (s->last_activation_tick > 0 && current_tick - s->last_activation_tick >= 10) {
        if (target->last_spike_tick < s->last_activation_tick) {
            s->weight -= learning_rate * 0.5f;
            s->last_activation_tick = 0; // Reset after depression
        }
    }

    // Homeostatic Scaling of Weights (Phase 23)
    if (target->spikes_in_last_window > 25) {
        s->weight -= 0.001f;
    } else if (target->spikes_in_last_window < 1 && s->weight < 10.0f) {
        s->weight += 0.0005f;
    }

    if (s->weight > 30.0f) s->weight = 30.0f; 
    if (s->weight < 0.1f) s->weight = 0.1f;

    // AXON DELIVERY & RECEPTOR BINDING (Spec 06 & 08)
    if (s->ticks_until_delivery >= 0) {
        if (s->ticks_until_delivery > 0) {
            s->ticks_until_delivery -= 1;
        }
        
        if (s->ticks_until_delivery == 0 && s->neurotransmitter_buffer > 0.0f) {
            
            // RECEPTOR SATURATION (Spec 08): Weight is the limit of chemical processing
            // DOPAMINE MODULATION (Spec 06): Dopamine scales the binding affinity
            float available_chemical = s->neurotransmitter_buffer * env.dopamine_multiplier;
            float receptor_limit = s->weight;
            
            float effective_signal = (available_chemical < receptor_limit) ? available_chemical : receptor_limit;
            
            s->last_activation_tick = current_tick;
            s->neurotransmitter_buffer -= (effective_signal / env.dopamine_multiplier); 
            
            float voltage_impact = effective_signal * 6.0f; // Scale constant
            
            if (s->is_inhibitory) {
                atomicAdd(&target->current_voltage, -voltage_impact);
            } else {
                atomicAdd(&target->current_voltage, voltage_impact);
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
    
    update_metabolism(&neurons[idx], env, current_tick);
    update_calcium(&neurons[idx]);
    process_spike(&neurons[idx], current_tick);
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
#ifdef USE_CUDA
    int t_per_b = 256;
    int n_blocks = (count + t_per_b - 1) / t_per_b;
    tick_neurons_kernel<<<n_blocks, t_per_b>>>(neurons, count, env, current_tick);
    cudaDeviceSynchronize(); 
    
    int s_blocks = (syn_count + t_per_b - 1) / t_per_b;
    tick_synapses_kernel<<<s_blocks, t_per_b>>>(synapses, syn_count, neurons, env, current_tick);
    cudaDeviceSynchronize();
#else
    for(int i = 0; i < count; ++i) {
        if (neurons[i].is_alive) {
            update_metabolism(&neurons[i], env, current_tick);
            update_calcium(&neurons[i]);
            process_spike(&neurons[i], current_tick);
        }
    }
    for(int i = 0; i < syn_count; ++i) {
        process_synapse(&synapses[i], &neurons[synapses[i].source_id], &neurons[synapses[i].target_id], env, current_tick);
    }
#endif
}

void init_neuron(BaseNeuron* neuron, uint32_t id, NeuronType type, uint32_t seed) {
    neuron->id = id;
    neuron->type = type;
    neuron->size = 1.0f; 
    neuron->is_alive = true;
    
    uint32_t local_seed = seed + id;

    // Heterogeneity: Gaussian distributions (Spec 11)
    float base_resting = RESTING_VOLTAGE;
    float base_threshold = THRESHOLD_VOLTAGE;
    
    switch(type) {
        case TYPE_PYRAMIDAL:
        case TYPE_BIPOLAR:
        case TYPE_INTERNEURON:
            break;
        case TYPE_PACEMAKER:
            base_threshold = -35.0f;
            break;
        case TYPE_PURKINJE:
            base_threshold = -45.0f;
            break;
        case TYPE_PHOTORECEPTOR:
            base_resting = -40.0f;
            base_threshold = 1000.0f; // Practical limit
            break;
        case TYPE_NOCICEPTOR:
            base_resting = -65.0f;
            base_threshold = -45.0f; 
            break;
    }

    neuron->resting_voltage = get_gaussian(&local_seed, base_resting, 4.0f); // Wider variance
    neuron->activation_threshold = get_gaussian(&local_seed, base_threshold, 2.5f);
    neuron->max_atp = get_gaussian(&local_seed, 100.0f, 10.0f);
    neuron->leak_coeff = get_gaussian(&local_seed, 1.2f, 0.2f);
    
    // Randomized structural properties
    neuron->size = get_gaussian(&local_seed, 1.0f, 0.2f);
    neuron->vesicles_pool = get_gaussian(&local_seed, 100.0f, 20.0f);

    neuron->energy_ATP = neuron->max_atp;
    neuron->current_voltage = neuron->resting_voltage;
    
    neuron->calcium_level = 0.0f;
    neuron->refractory_time_left = 0.0f;
    neuron->last_spike_tick = 0;
    neuron->spikes_in_last_window = 0;
    neuron->metabolic_wear = 0.0f;
}

} // extern "C"
