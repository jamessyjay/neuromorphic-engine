// cuda/base_neuron.cu

#include "base_neuron.h"
#include <cmath>
#include <cstdint>

// --- PHASE 13: ADVANCED BIOLOGY (Heterogeneity & evolution) ---

// Simple PRNG for Mock CPU (Xorshift)
static uint32_t xorshift_seed = 42;
static float get_random_float_01() {
    xorshift_seed ^= xorshift_seed << 13;
    xorshift_seed ^= xorshift_seed >> 17;
    xorshift_seed ^= xorshift_seed << 5;
    return (float)(xorshift_seed % 1000) / 1000.0f;
}

// Simple Gaussian Approximation (Box-Muller)
static float get_gaussian(float mu, float sigma) {
    float u1 = get_random_float_01();
    float u2 = get_random_float_01();
    if (u1 < 0.0001f) u1 = 0.0001f;
    float z = (float)sqrt(-2.0 * log(u1)) * (float)cos(2.0 * 3.14159 * u2);
    return mu + sigma * z;
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
static inline __device__ void update_metabolism(BaseNeuron* n, Environment env) {
    if (n->energy_ATP <= 0.0f) {
        die_gracefully(n);
        return;
    }

    // Step 1: ATP Generation (Affected by oxygen and WEAR)
    // Metabolic Stress (Spec 11): 
    // If ATP is critically low, mitochondria wear out faster
    if (n->energy_ATP < 20.0f) {
        n->metabolic_wear += 0.0001f; // Slow permanent damage
        if (n->metabolic_wear > 0.9f) n->metabolic_wear = 0.9f;
    }

    if (n->energy_ATP < n->max_atp) {
        // Efficiency falls with wear
        float efficiency = (1.0f - n->metabolic_wear);
        float generated_atp = 0.9f * env.global_oxygen_level * efficiency;
        n->energy_ATP += generated_atp; 
        if (n->energy_ATP > n->max_atp) n->energy_ATP = n->max_atp;
    }

    // BASELINE LEAKAGE (Maintenance cost for keeping the gradient)
    n->energy_ATP -= 0.05f * n->leak_coeff;

    // Step 2: Homeostatic Plasticity (Spec 11)
    // If neuron fires too much, it becomes "thick-skinned" (increase threshold)
    // Periodically we'd reset this counter, but for MVP we scale based on accumulation
    if (n->spikes_in_last_window > 50) {
        n->activation_threshold += 0.01f; // Desensitization
        if (n->activation_threshold > 0.0f) n->activation_threshold = 0.0f;
        // Slow recovery of threshold if not spiking?
    } else if (n->spikes_in_last_window < 5 && n->activation_threshold > n->resting_voltage + 10.0f) {
        n->activation_threshold -= 0.005f; // Sensitization
    }

    // Reset window every 1000 ticks? (Simplification: slow decay of counter)
    if (n->spikes_in_last_window > 0) {
        n->spikes_in_last_window = 0; // Reset for next tick logic in orchestrator? 
        // No, let's keep it cumulative for now but slow decay.
    }

    // Step 3: Voltage drift
    if (n->type == TYPE_PACEMAKER) {
        n->current_voltage += 0.5f; 
        n->energy_ATP -= 0.1f * n->leak_coeff;
    } else {
        if (n->current_voltage > n->resting_voltage) {
            n->current_voltage -= 0.1f * n->leak_coeff; 
            n->energy_ATP -= 0.1f * n->leak_coeff;
            if (n->current_voltage < n->resting_voltage) n->current_voltage = n->resting_voltage;
        } else if (n->current_voltage < n->resting_voltage) {
            n->current_voltage += 0.1f * n->leak_coeff;
            n->energy_ATP -= 0.1f * n->leak_coeff;
        }
    }
}

static inline __device__ void update_calcium(BaseNeuron* n) {
    if (n->calcium_level > 0.0f) {
        n->calcium_level -= 1.0f; 
        if (n->calcium_level < 0.0f) n->calcium_level = 0.0f;
    }
    if (n->calcium_level >= FATAL_CALCIUM_LEVEL) die_gracefully(n);
}

static inline __device__ void process_spike(BaseNeuron* n, uint32_t current_tick) {
    if (n->refractory_time_left > 0.0f) {
        n->refractory_time_left -= 1.0f; 
        if (n->refractory_time_left >= REFRACTORY_PERIOD_RELATIVE_MS) {
            n->current_voltage = -80.0f;
            return; 
        }
        float temp_threshold = -40.0f;
        if (n->current_voltage >= temp_threshold) {
             n->current_voltage = SPIKE_VOLTAGE;
             n->refractory_time_left = (REFRACTORY_PERIOD_ABSOLUTE_MS + REFRACTORY_PERIOD_RELATIVE_MS) * 1.0f;
             n->last_spike_tick = current_tick;
             n->energy_ATP -= ATP_SPIKE_COST;
             n->calcium_level += CALCIUM_SPIKE_GAIN;
             n->spikes_in_last_window++;
             if (n->vesicles_pool >= 1.0f) n->vesicles_pool -= 1.0f;
        } else {
             if (n->current_voltage > n->resting_voltage) n->current_voltage = -80.0f; 
        }
        return; 
    }
    if (n->current_voltage >= n->activation_threshold) {
        if (n->type == TYPE_PHOTORECEPTOR) return;
        n->current_voltage = SPIKE_VOLTAGE;
        n->refractory_time_left = (REFRACTORY_PERIOD_ABSOLUTE_MS + REFRACTORY_PERIOD_RELATIVE_MS) * 1.0f;
        n->last_spike_tick = current_tick;
        n->energy_ATP -= ATP_SPIKE_COST;
        n->calcium_level += CALCIUM_SPIKE_GAIN;
        n->spikes_in_last_window++;
        if (n->vesicles_pool >= 1.0f) n->vesicles_pool -= 1.0f;
        
        if (n->type == TYPE_NOCICEPTOR) {
            n->activation_threshold -= 1.0f;
            if (n->activation_threshold < -60.0f) n->activation_threshold = -60.0f;
        }
    }
}

static inline __device__ void process_synapse(Synapse* s, BaseNeuron* source, BaseNeuron* target, Environment env, uint32_t current_tick) {
    if (source->is_alive) {
        int delay = s->is_myelinated ? 0 : 1;
        if (source->type == TYPE_PHOTORECEPTOR) {
            if (source->current_voltage >= -41.0f) {
                s->neurotransmitter_buffer += 1.0f;
                s->ticks_until_delivery = delay;
            }
        } else {
            if (source->last_spike_tick == current_tick && source->current_voltage == SPIKE_VOLTAGE) {
                s->neurotransmitter_buffer += 20.0f; 
                s->ticks_until_delivery = delay;
            }
        }
    }
    if (s->neurotransmitter_buffer > 0.0f) {
        s->neurotransmitter_buffer -= 0.1f;
        if (s->neurotransmitter_buffer < 0.0f) s->neurotransmitter_buffer = 0.0f;
    }
    if (s->last_activation_tick > 0 && target->last_spike_tick > s->last_activation_tick) {
        if (target->last_spike_tick - s->last_activation_tick <= 20) {
            // PHASE 14: Dopamine Modulated Learning
            // Weight growth is scaled by dopamine. No dopamine = no learning.
            float learning_rate = 0.01f * env.dopamine_multiplier;
            s->weight += learning_rate;
            if (s->weight > 20.0f) s->weight = 20.0f; // Soft cap
            s->last_activation_tick = 0;
        }
    }
    if (s->ticks_until_delivery > 0) {
        s->ticks_until_delivery -= 1;
        if (s->ticks_until_delivery == 0 && s->neurotransmitter_buffer > 0.0f) {
            float effective_signal = s->neurotransmitter_buffer * env.dopamine_multiplier;
            if (effective_signal > s->weight) effective_signal = s->weight;
            s->last_activation_tick = current_tick;
            s->neurotransmitter_buffer -= (effective_signal / env.dopamine_multiplier); 
            if (s->is_inhibitory) {
                atomicAdd(&target->current_voltage, -(effective_signal * 5.0f));
            } else {
                atomicAdd(&target->current_voltage, (effective_signal * 5.0f));
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
    
    // Step 1: Handle Metabolism and Spikes (Neuron-Parallel)
    int n_blocks = (count + t_per_b - 1) / t_per_b;
    tick_neurons_kernel<<<n_blocks, t_per_b>>>(neurons, count, env, current_tick);
    cudaDeviceSynchronize(); // Wait for spikes to be determined before synapse processing
    
    // Step 2: Handle Synapses (Synapse-Parallel, scale to 100k+)
    int s_blocks = (syn_count + t_per_b - 1) / t_per_b;
    tick_synapses_kernel<<<s_blocks, t_per_b>>>(synapses, syn_count, neurons, env, current_tick);
    cudaDeviceSynchronize();
#else
    // HIGH-PERFORMANCE MOCK: Single-loop iteration for CPU
    // We avoid calling kernels here to minimize stack overhead and allow aggressive inlining
    for(int i = 0; i < count; ++i) {
        if (neurons[i].is_alive) {
            update_metabolism(&neurons[i], env);
            update_calcium(&neurons[i]);
            process_spike(&neurons[i], current_tick);
        }
    }
    for(int i = 0; i < syn_count; ++i) {
        process_synapse(&synapses[i], &neurons[synapses[i].source_id], &neurons[synapses[i].target_id], env, current_tick);
    }
#endif
}

void init_neuron(BaseNeuron* neuron, uint32_t id, NeuronType type) {
    neuron->id = id;
    neuron->type = type;
    neuron->size = 1.0f; 
    neuron->is_alive = true;
    
    // Seed randomization based on ID to have deterministic but diverse neurons
    xorshift_seed = 42 + id;

    // Heterogeneity: Gaussian distributions (Spec 11)
    float base_resting = RESTING_VOLTAGE;
    float base_threshold = THRESHOLD_VOLTAGE;
    
    switch(type) {
        case TYPE_PYRAMIDAL:
        case TYPE_BIPOLAR:
        case TYPE_INTERNEURON:
            break;
        case TYPE_PACEMAKER:
            base_threshold = -40.0f;
            break;
        case TYPE_PURKINJE:
            base_threshold = -50.0f;
            break;
        case TYPE_PHOTORECEPTOR:
            base_resting = -40.0f;
            base_threshold = 1000.0f; // Practically unreachable
            break;
        case TYPE_NOCICEPTOR:
            base_resting = -65.0f;
            base_threshold = -50.0f; // Dynamic
            break;
    }

    neuron->resting_voltage = get_gaussian(base_resting, 2.5f);
    neuron->activation_threshold = get_gaussian(base_threshold, 1.5f);
    neuron->max_atp = get_gaussian(100.0f, 5.0f);
    neuron->leak_coeff = get_gaussian(1.0f, 0.1f);
    
    neuron->energy_ATP = neuron->max_atp;
    neuron->current_voltage = neuron->resting_voltage;
    
    neuron->calcium_level = 0.0f;
    neuron->vesicles_pool = 100.0f;
    neuron->refractory_time_left = 0.0f;
    neuron->last_spike_tick = 0;
    neuron->spikes_in_last_window = 0;
    neuron->metabolic_wear = 0.0f;
    
    neuron->inputs_count = 0;
    neuron->outputs_count = 0;
}

} // extern "C"
