// cuda/base_neuron.h
// C++ / CUDA Definitions for Neuromorphic Single Neuron

#pragma once

#include <stdint.h>
#include <stdbool.h>

// Cross-platform mock macros for CPU execution on macOS
#ifndef USE_CUDA
    #define __global__
    #define __device__
    #define __host__
    // Mock atomicAdd for CPU testing - simplified for speed
    #define atomicAdd(addr, val) (*(addr) += (val))
#endif

extern "C" {

// Physical Constants (From specs/02_neuron_spec.md & 07_network_and_limits.md)
#define RESTING_VOLTAGE -70.0f
#define THRESHOLD_VOLTAGE -55.0f
#define SPIKE_VOLTAGE 40.0f
#define REFRACTORY_PERIOD_ABSOLUTE_MS 1.0f
#define REFRACTORY_PERIOD_RELATIVE_MS 3.0f

#define MAX_ATP 100.0f
#define ATP_SPIKE_COST 20.0f
#define CALCIUM_SPIKE_GAIN 10.0f
#define FATAL_CALCIUM_LEVEL 100.0f

// Base C-struct matching the BaseNeuron specification (specs/05_oop_architecture.md).
// Extended with Biology specs (specs/02_neuron_spec.md)

// Data-Oriented OOP equivalent (Phase 5 & 8)
typedef enum {
    TYPE_PYRAMIDAL = 0,
    TYPE_BIPOLAR = 1,
    TYPE_INTERNEURON = 2,
    TYPE_PACEMAKER = 3,
    TYPE_PURKINJE = 4,
    TYPE_PHOTORECEPTOR = 5,
    TYPE_NOCICEPTOR = 6
} NeuronType;

typedef struct {
    uint32_t id;
    NeuronType type;
    float size;
    float current_voltage;
    
    // Adaptive thresholds
    float activation_threshold;  
    float resting_voltage;

    float energy_ATP;
    float max_atp;
    float leak_coeff;
    
    // Biological States
    float calcium_level;
    float vesicles_pool;
    float refractory_time_left;
    
    // Evolution & Adaptation
    uint32_t spikes_in_last_window;
    float metabolic_wear;
    uint32_t last_spike_tick;

    bool is_alive;
} BaseNeuron;

// Environment Singleton Representation
typedef struct {
    float global_oxygen_level;    // 0.0 to 1.0
    float global_glucose_level;   // 0.0 to 1.0 (Phase 26)
    float dopamine_multiplier;    // > 1.0 boosts signal
    float toxin_levels;           
    float light_level;            
    uint32_t seed;                
} Environment;

// Synaptic buffer representation (Packed: 24 bytes)
typedef struct {
    uint32_t source_id;
    uint32_t target_id;
    float weight;
    float neurotransmitter_buffer;
    uint32_t last_activation_tick;
    uint8_t ticks_until_delivery;
    bool is_inhibitory;
    bool is_myelinated;
} Synapse;

// Orchestrator Function declarations
void launch_tick_neurons(BaseNeuron* neurons, int count, Synapse* synapses, int syn_count, Environment env, uint32_t current_tick);
void init_neuron(BaseNeuron* neuron, uint32_t id, NeuronType type, uint32_t seed);

} // extern "C"
