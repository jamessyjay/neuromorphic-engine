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
#define MAX_SYNAPSE_WEIGHT 30.0f

#define DEATH_NONE 0
#define DEATH_ATP_LOW 1
#define DEATH_CALCIUM_HIGH 2
#define DEATH_GLUCOSE_STARVATION 3

// Dendritic Tree Physics (Phase 33/34)
#define BRANCH_LEAK_COEFF 0.9f
#define NMDA_SPIKE_GAIN 5.0f
#define TRUNK_MULTIPLIER 2.0f
#define TRUNK_ATP_COST 5.0f
#define TUFT_MULTIPLIER 0.5f

// Metabolic & Homeostatic Constants
#define METABOLIC_FLOOR 0.15f
#define EMERGENCY_ATP_SILENCE 20.0f
#define STDP_LEARNING_RATE 0.1f
#define VESICLE_SPIKE_COST 5.0f
#define AGING_WEAR_INC 0.0005f
#define RECEPTOR_SCALE_CONSTANT 6.0f

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
    uint8_t branch_type;      // 1 byte: 0=Empty, 1=Basal, 2=Trunk, 3=Tuft
    uint8_t refractory_timer; // 1 byte: Cooldown ticks for the branch
    uint16_t voltage;         // 2 bytes: Fixed-point (-100 to +100 mV)
    uint16_t nmda_threshold;  // 2 bytes: Fixed-point
    uint8_t _padding[2];      // 2 bytes: Align to 8 bytes total
} Compartment;

typedef struct alignas(64) {
    uint32_t id;
    uint8_t type;             // 1 byte: NeuronType
    uint8_t death_reason;     // 1 byte
    bool is_alive;            // 1 byte
    uint8_t _pad1;            // 1 byte
    
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
    uint32_t last_spike_tick;
    uint16_t spikes_in_last_window;
    uint16_t metabolic_wear; // Packed 0.0-1.0 to 0-65535

    uint16_t x, y, z;        // Packed 0.0-100.0 to 0-65535
    uint16_t _pad2;          // Align to 8 bytes for the base struct before branches
    
    // Dendritic Tree (Phase 33)
    Compartment branches[8]; // 8 * 8 = 64 bytes
} BaseNeuron; // Total: 64 (base) + 64 (branches) = 128 bytes (exactly 2 cache lines)

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
    uint32_t ticks_below_threshold;
    uint8_t ticks_until_delivery;
    uint8_t flags; // Bits 0-2: branch_id, Bit 3: is_inhibitory, Bit 4: is_myelinated
    uint8_t _padding[2];
} Synapse;

// Orchestrator Function declarations
void launch_tick_neurons(BaseNeuron* neurons, int count, Synapse* synapses, int syn_count, Environment env, uint32_t current_tick);
void init_neuron(BaseNeuron* neuron, uint32_t id, NeuronType type, uint32_t seed);

} // extern "C"
