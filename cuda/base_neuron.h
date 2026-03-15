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
    #ifdef __cplusplus
    #include <atomic>
    // Thread-safe atomicAdd for CPU testing using C++ std::atomic
    inline void atomicAdd(float* addr, float val) {
        std::atomic<float>* a = reinterpret_cast<std::atomic<float>*>(addr);
        float current = a->load(std::memory_order_relaxed);
        while (!a->compare_exchange_weak(current, current + val, std::memory_order_release, std::memory_order_relaxed));
    }
#else
    #define atomicAdd(addr, val) (*(addr) += (val))
#endif
#endif

extern "C" {

// Physical Constants (From specs/02_neuron_spec.md & 07_network_and_limits.md)
// --- Magic Numbers extracted from Phase 35 Cleanup ---
#define MAX_U16_FLOAT 65535.0f
#define VOLTAGE_SHIFT 100.0f
#define VOLTAGE_SCALE 100.0f
#define MAX_VESICLES 100.0f

#define NEURON_RESTING_VOLTAGE -70.0f
#define NEURON_THRESHOLD -55.0f
#define ACTION_POTENTIAL_PEAK 40.0f

// Metabolic parameters (Phase 40)
#define ATP_RECOVERY_RATE 0.05f
#define ATP_SPIKE_COST 25.0f

// Refractory and Recovery
#define ABSOLUTE_REFRACTORY_PERIOD 2.0f
#define VOLTAGE_DECAY_RATE 0.1f // Membrane leak
#define CALCIUM_SPIKE_GAIN 5.0f
#define FATAL_CALCIUM_LEVEL 1000.0f
#define MAX_SYNAPSE_WEIGHT 5.0f

#define DEATH_NONE 0
#define DEATH_ATP_LOW 1
#define DEATH_CALCIUM_HIGH 2
#define DEATH_GLUCOSE_STARVATION 3

// Dendritic Tree Physics (Phase 33/34)
#define BRANCH_LEAK_COEFF 0.9f
// MMN v1.1: NMDA logical AND gate multiplier. Applied when a branch exceeds its nmda_threshold.
#define NMDA_SPIKE_GAIN 5.0f
#define TRUNK_MULTIPLIER 2.0f
#define TRUNK_ATP_COST 5.0f
#define TUFT_MULTIPLIER 0.5f

// Metabolic & Homeostatic Constants
#define METABOLIC_FLOOR 0.6f
#define EMERGENCY_ATP_SILENCE 20.0f
#define STDP_LEARNING_RATE 3.0f
#define VESICLES_PER_SPIKE 1.0f
#define AGING_WEAR_INC 0.0f
#define RECEPTOR_SCALE_CONSTANT 40.0f

// Additional Homeostatic Constants
#define HOMEOSTATIC_SPIKE_HIGH 20
#define HOMEOSTATIC_SPIKE_LOW 2
#define HOMEOSTATIC_PENALTY 0.5f
#define HOMEOSTATIC_RECOVERY 0.1f

#define BASELINE_LEAK_RATE 0.01f
#define LOW_ATP_LEAK_MULTIPLIER 1.1f
#define ATP_STRESS_THRESHOLD 30.0f
// MMN v1.1: Hunger sensitivity. Controls how much the activation threshold increases when ATP is low.
#define ATP_HUNGER_BETA 0.5f
#define EMERGENCY_ATP_THRESHOLD 0.01f
#define EMERGENCY_SILENCE_THRESHOLD 0.0f // Achievable mV, but high (sleep)
#define CALCIUM_RECOVERY_RATE 0.5f
#define VOLTAGE_DRIFT_RATE 0.4f

// Photoreceptor specific
#define PHOTO_HYPERPOLARIZED -65.0f
#define PHOTO_DARK_CURRENT -40.0f
#define PHOTO_ACTIVATION_SPEED 0.2f
#define PHOTO_CONSTANT_LEAK_THRESHOLD -41.0f

// Specialized Cells
#define PACEMAKER_DEPOLARIZATION 3.5f
#define NOCICEPTOR_SENSITIZATION -2.0f
#define NOCICEPTOR_MIN_THRESHOLD -60.0f

// Synaptic Dynamics
#define BRANCH_REFRACTORY_TICKS 3
#define BASAL_SOMA_GAIN 0.1f
#define TRUNK_PASSIVE_COST_MULTIPLIER 0.1f
#define PASSIVE_LEAK_PERCENT 0.05f
#define MAX_METABOLIC_WEAR 0.98f
#define HYPERPOLARIZATION_OFFSET 10.0f
#define RELATIVE_REFRACTORY_MULTIPLIER 1.5f

// STDP (Spike-Timing-Dependent Plasticity) Parameters (Phase 45)
#define STDP_TAU_PLUS 20.0f
#define STDP_TAU_MINUS 20.0f
#define STDP_A_PLUS 0.005f
#define STDP_A_MINUS 0.007f
#define STDP_DECAY 0.90f
#define MIN_SYNAPSE_WEIGHT 0.01f

#define CONSTANT_PHOTORECEPTOR_LEAK 2.0f
#define BULK_NEUROTRANSMITTER_RELEASE 25.0f
#define LTP_TIME_WINDOW 20
#define LTD_TIME_WINDOW 10
#define LTD_PENALTY_MULTIPLIER 0.5f

#define SYNAPSE_SCALING_HIGH 25
#define SYNAPSE_SCALING_LOW 1
#define SYNAPSE_PENALTY 0.05f
#define SYNAPSE_RECOVERY 0.01f
#define SYNAPSE_RECOVERY_MAX_WEIGHT 5.0f
#define SYNAPSE_RELIABILITY_THRESHOLD 0.5f
#define SYNAPSE_RELIABILITY_PENALTY 0.8f

// Structural Defaults
#define DEFAULT_VOLTAGE_VAR 2.0f
#define PURKINJE_RESTING -65.0f
#define PURKINJE_THRESHOLD -50.0f
#define NOCICEPTOR_RESTING -70.0f
#define DEFAULT_ATP_VAR 10.0f
#define DEFAULT_LEAK_VAR 0.2f
#define DEFAULT_SIZE_VAR 0.2f
#define DEFAULT_VESICLES_VAR 20.0f
#define DEFAULT_COORD_VAR 20.0f
#define DEFAULT_COORD_CENTER 50.0f
#define MAX_COORD 100.0f

// Initialization & Defaults
#define INIT_LEAK_COEFF_MEAN 1.2f
#define INIT_SIZE_MEAN 1.0f

#define PYRAMIDAL_THRESHOLD -52.0f
#define INTERNEURON_RESTING -65.0f
#define INTERNEURON_THRESHOLD -50.0f
#define PACEMAKER_RESTING -60.0f
#define PACEMAKER_THRESHOLD -55.0f
#define PHOTO_THRESHOLD 0.0f
#define NOCICEPTOR_THRESHOLD -45.0f

// Runtime configurations
#define NUM_BRANCHES 8
#define THREADS_PER_BLOCK 256
#define MYELINATED_DELIVERY_TICKS 0
#define UNMYELINATED_DELIVERY_TICKS 2
#define RANDOM_MODULO_BASE 100

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
    uint8_t local_trace;      // 1 byte: Dendritic STDP trace
    uint8_t _pad;             // 1 byte: Align to 8 bytes total
} Compartment;

typedef struct alignas(64) {
    uint32_t id;
    uint8_t type;             // 1 byte: NeuronType
    uint8_t death_reason;     // 1 byte
    uint8_t is_alive;         // 1 byte (1=true, 0=false)            // 1 byte
    uint8_t internal_phase;   // 1 byte: Internal clock for phase resonance
    
    float spike_trace;
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
    uint8_t spikes_in_last_window;
    uint8_t predictive_ticks;
    uint16_t metabolic_wear; // Packed 0.0-1.0 to 0-65535

    uint16_t x, y, z;        // Packed 0.0-100.0 to 0-65535
    uint16_t dead_ticks;     // Replaces _pad2 for Phoenix loop
    
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
    float stdp_learning_rate;
    float wakefulness_multiplier;                
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

#ifdef __cplusplus
extern "C" {
#endif

void cleanup_gpu_resources(void* ptr);
void synchronize_device();

#ifdef __cplusplus
}
#endif
