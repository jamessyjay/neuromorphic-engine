// cuda/base_neuron.cu

#include "base_neuron.h"

#include "base_neuron.h"

// Atomic function: Apoptosis / Death Handler
__device__ void die_gracefully(BaseNeuron* n) {
    n->is_alive = false;
    n->current_voltage = 0.0f; // Dead cell voltage drops to neutral
    n->energy_ATP = 0.0f;
    n->calcium_level = 0.0f;
    n->refractory_time_left = 0.0f;
}

// Atomic function: handles leak rate and mitochondria (ATP usage), affected by oxygen
__device__ void update_metabolism(BaseNeuron* n, Environment env) {
    if (n->energy_ATP <= 0.0f) {
        die_gracefully(n); // Trigger apoptosis condition
        return;
    }
    
    // Mitochondria generate ATP (+1.0) multiplied by global oxygen level
    if (n->energy_ATP < MAX_ATP) {
        float generated_atp = 0.9f * env.global_oxygen_level;
        n->energy_ATP += generated_atp; 
        if (n->energy_ATP > MAX_ATP) n->energy_ATP = MAX_ATP;
    }

    if (n->type == TYPE_PACEMAKER) {
        // Pacemaker autonomously drifts towards threshold
        n->current_voltage += 0.5f; 
        n->energy_ATP -= 0.1f;
    } else {
        // Sodium-Potassium pumps working to restore resting_voltage
        if (n->current_voltage > n->resting_voltage) {
            n->current_voltage -= 0.1f; 
            n->energy_ATP -= 0.1f; // Costs energy to pump ions
            
            if (n->current_voltage < n->resting_voltage) {
                n->current_voltage = n->resting_voltage;
            }
        } else if (n->current_voltage < n->resting_voltage) {
            n->current_voltage += 0.1f;
            n->energy_ATP -= 0.1f;
        }
    }
}

// Atomic function: Calcium apoptosis rule
__device__ void update_calcium(BaseNeuron* n) {
    // Calcium slowly clears out over time (Regeneration)
    if (n->calcium_level > 0.0f) {
        n->calcium_level -= 1.0f; 
        if (n->calcium_level < 0.0f) n->calcium_level = 0.0f;
    }
    // Excitotoxicity
    if (n->calcium_level >= FATAL_CALCIUM_LEVEL) {
        die_gracefully(n);
    }
}

// Atomic function: Fire Spike and Refractory mapping
__device__ void process_spike(BaseNeuron* n, uint32_t current_tick) {
    if (n->refractory_time_left > 0.0f) {
        n->refractory_time_left -= 1.0f; 
        
        // Absolute Cooldown (First 1ms)
        if (n->refractory_time_left >= REFRACTORY_PERIOD_RELATIVE_MS) {
            n->current_voltage = -80.0f; // Completely dead/hyperpolarized
            return; 
        }
        
        // Relative Cooldown (Next 3ms): Threshold is temporally higher than normal
        // e.g., instead of -55mV, it might take -40mV to fire again
        float temp_threshold = -40.0f;
        if (n->current_voltage >= temp_threshold) {
             // Overcame the relative refractory period!
             n->current_voltage = SPIKE_VOLTAGE;
             n->refractory_time_left = (REFRACTORY_PERIOD_ABSOLUTE_MS + REFRACTORY_PERIOD_RELATIVE_MS) * 1.0f;
             n->last_spike_tick = current_tick;
             n->energy_ATP -= ATP_SPIKE_COST;
             n->calcium_level += CALCIUM_SPIKE_GAIN;
             if (n->vesicles_pool >= 1.0f) {
                 n->vesicles_pool -= 1.0f;
             }
        } else {
             // Slowly recovering voltage to resting state during relative refractory
             if (n->current_voltage > n->resting_voltage) {
                 n->current_voltage = -80.0f; 
             }
        }
        
        return; 
    }

    // Normal State Firing
    if (n->current_voltage >= n->activation_threshold) {
        // Photoreceptor disabled override - it never fires spikes!
        if (n->type == TYPE_PHOTORECEPTOR) return;

        // Fire Spike!
        n->current_voltage = SPIKE_VOLTAGE;
        n->refractory_time_left = (REFRACTORY_PERIOD_ABSOLUTE_MS + REFRACTORY_PERIOD_RELATIVE_MS) * 1.0f;
        n->last_spike_tick = current_tick;
        
        // Firing takes massive energy and opens calcium channels
        n->energy_ATP -= ATP_SPIKE_COST;
        n->calcium_level += CALCIUM_SPIKE_GAIN;
        
        // Neurotransmitter release logic will go here
        if (n->vesicles_pool >= 1.0f) {
            n->vesicles_pool -= 1.0f;
        }

        // Nociceptor Dynamic Threshold (Spec 05): more spikes = lower threshold (Pain sensitization)
        if (n->type == TYPE_NOCICEPTOR) {
            n->activation_threshold -= 1.0f;
            if (n->activation_threshold < -60.0f) n->activation_threshold = -60.0f; // Cap at -60mV
        }
    }
}

// Atomic function: Synaptic transmission, Dopamine logic and STDP Hebbian learning
__device__ void process_synapse(Synapse* s, BaseNeuron* source, BaseNeuron* target, Environment env, uint32_t current_tick) {
    
    // --- AXON DELIVERY (Source -> Synapse) ---
    if (source->is_alive) {
        int delay = s->is_myelinated ? 0 : 1; // SchwannCell teleport

        // Photoreceptor analog logic
        if (source->type == TYPE_PHOTORECEPTOR) {
            // Dark state (-40mV) -> Constant stream of neurotransmitters
            // Light state (-65mV) -> Stops stream
            if (source->current_voltage >= -41.0f) {
                s->neurotransmitter_buffer += 1.0f;
                s->ticks_until_delivery = delay;
            }
        } else {
            // Normal Spiking Neuron Logic
            if (source->last_spike_tick == current_tick && source->current_voltage == SPIKE_VOLTAGE) {
                // If it spiked THIS EXACT TICK, push a package into buffer
                s->neurotransmitter_buffer += 20.0f; 
                s->ticks_until_delivery = delay;
            }
        }
    }

    // --- RECEPTOR BINDING (Synapse -> Target) ---
    
    // Astrocyte emulation: neurotransmitter degrades over time
    if (s->neurotransmitter_buffer > 0.0f) {
        s->neurotransmitter_buffer -= 0.1f; // Slow degradation
        if (s->neurotransmitter_buffer < 0.0f) s->neurotransmitter_buffer = 0.0f;
    }

    // Hebbian STDP Learning: Check if target fired RECENTLY AFTER this synapse was activated
    // Strict STDP limits: Target must spike within 20 ticks AFTER synapse delivered molecules
    if (s->last_activation_tick > 0 && target->last_spike_tick > s->last_activation_tick) {
        if (target->last_spike_tick - s->last_activation_tick <= 20) {
            s->weight += 0.01f; // Grow receptors! Permanent structural plasticity
            s->last_activation_tick = 0; // Reset to avoid double-counting
        }
    }

    if (s->ticks_until_delivery > 0) {
        s->ticks_until_delivery -= 1;
        
        // Deliver payload when delay is over
        if (s->ticks_until_delivery == 0 && s->neurotransmitter_buffer > 0.0f) {
            
            // Dopamine Modulator: Multiplies effectiveness of the neurotransmitter (Spec 06)
            float effective_signal = s->neurotransmitter_buffer * env.dopamine_multiplier;
            
            // Receptor Saturation: Effective Signal = MIN(Incoming, Receptor_Count(weight))
            if (effective_signal > s->weight) {
                effective_signal = s->weight; // Saturated
            }
            
            // Mark activation for STDP Hebbian tracking
            s->last_activation_tick = current_tick;
            
            // Only consume the amount passed through receptors (without the dopamine multiplier)
            s->neurotransmitter_buffer -= (effective_signal / env.dopamine_multiplier); 
            
            if (s->is_inhibitory) {
                target->current_voltage -= effective_signal * 5.0f; // GABA reduces firing chance
            } else {
                target->current_voltage += effective_signal * 5.0f; // Glutamate mapped to voltage
            }
        }
    }
}

// Orchestrator function (Global Kernel)
__global__ void tick_neurons(BaseNeuron* neurons, int count, Synapse* synapses, int syn_count, Environment env, uint32_t current_tick) {
#ifdef USE_CUDA
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
#else
    for(int idx = 0; idx < count; ++idx) {
#endif

        if (idx >= count) return;
        if (!neurons[idx].is_alive) return;

        update_metabolism(&neurons[idx], env);
        update_calcium(&neurons[idx]);
        process_spike(&neurons[idx], current_tick);

#ifndef USE_CUDA
    } 
#endif

    // Process Synapses sequentially for mock CPU
#ifndef USE_CUDA
    for(int s_idx = 0; s_idx < syn_count; ++s_idx) {
        process_synapse(&synapses[s_idx], &neurons[synapses[s_idx].source_id], &neurons[synapses[s_idx].target_id], env, current_tick);
    }
#endif
}

extern "C" {

void launch_tick_neurons(BaseNeuron* neurons, int count, Synapse* synapses, int syn_count, Environment env, uint32_t current_tick) {
#ifdef USE_CUDA
    int threads_per_block = 256;
    int blocks = (count + threads_per_block - 1) / threads_per_block;
    tick_neurons<<<blocks, threads_per_block>>>(neurons, count, synapses, syn_count, env, current_tick);
    cudaDeviceSynchronize();
#else
    tick_neurons(neurons, count, synapses, syn_count, env, current_tick);
#endif
}

void init_neuron(BaseNeuron* neuron, uint32_t id, NeuronType type) {
    neuron->id = id;
    neuron->type = type;
    neuron->size = 1.0f; 
    neuron->energy_ATP = 100.0f;
    neuron->is_alive = true;
    
    switch(type) {
        case TYPE_PYRAMIDAL:
        case TYPE_BIPOLAR:
        case TYPE_INTERNEURON:
            neuron->resting_voltage = RESTING_VOLTAGE;
            neuron->activation_threshold = THRESHOLD_VOLTAGE;
            break;
        case TYPE_PACEMAKER:
            neuron->resting_voltage = RESTING_VOLTAGE;
            neuron->activation_threshold = -40.0f;
            break;
        case TYPE_PURKINJE:
            neuron->resting_voltage = RESTING_VOLTAGE;
            neuron->activation_threshold = -50.0f;
            break;
        case TYPE_PHOTORECEPTOR:
            neuron->resting_voltage = -40.0f;
            neuron->activation_threshold = 1000.0f; // Practically unreachable
            break;
        case TYPE_NOCICEPTOR:
            neuron->resting_voltage = -65.0f;
            neuron->activation_threshold = -50.0f; // Dynamic, drops over time
            break;
    }

    neuron->current_voltage = neuron->resting_voltage;
    
    neuron->calcium_level = 0.0f;
    neuron->vesicles_pool = 100.0f;
    neuron->refractory_time_left = 0.0f;
    neuron->last_spike_tick = 0;
    
    neuron->inputs_count = 0;
    neuron->outputs_count = 0;
}

} // extern "C"
