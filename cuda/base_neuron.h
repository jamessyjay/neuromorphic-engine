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
    
    // Adaptive thresholds based on NeuronType
    float activation_threshold;  
    float resting_voltage;

    float energy_ATP;            // Current ATP
    float max_atp;               // Genetic limit (Heterogeneity)
    float leak_coeff;            // Efficiency of pumps (Heterogeneity)
    bool is_alive;
    
    // Phase 2 & 13: Biological States
    float calcium_level;         // Internal Calcium
    float vesicles_pool;         // Amount of neurotransmitter packages ready for release
    float refractory_time_left;  // Time left in MS for absolute/relative cooldown
    
    // Evolution & Adaptation (Phase 13)
    uint32_t spikes_in_last_window; // Counter for Homeostatic Plasticity
    float metabolic_wear;           // 0.0 (new) to 1.0 (dead mitochondria)

    // Phase 3: Synapse and Output Tracking
    uint32_t last_spike_tick;    // For Hebbian learning (STDP)

    // Arrays simulated as counts for the MVP
    uint32_t inputs_count;
    uint32_t outputs_count;
} BaseNeuron;

// Environment Singleton Representation
typedef struct {
    float global_oxygen_level;    // 0.0 to 1.0 (multiplier for ATP generation)
    float dopamine_multiplier;    // 1.0 is baseline, > 1.0 boosts Glutamate/GABA strength
    float toxin_levels;           // (Future use)
} Environment;

// Synaptic buffer representation (Specs 07 & 05)
typedef struct {
    uint32_t source_id;
    uint32_t target_id;
    float weight;         // receptor_count multiplier
    float neurotransmitter_buffer;
    int ticks_until_delivery;
    bool is_inhibitory;   // if true -> emits GABA (subtractive), else Glutamate (additive)
    
    // Hebbian STDP Tracker
    uint32_t last_activation_tick; // Used to track exact moment Glutamate hits target
    
    // Schwann Cell / Myelin sheath
    bool is_myelinated;   // If true -> delay is 0 (teleports)
} Synapse;

// Orchestrator Function declarations
void launch_tick_neurons(BaseNeuron* neurons, int count, Synapse* synapses, int syn_count, Environment env, uint32_t current_tick);
void init_neuron(BaseNeuron* neuron, uint32_t id, NeuronType _type);

} // extern "C"
