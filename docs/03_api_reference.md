# Neuromorphic Engine: API & FFI Reference

Technical description of interaction interfaces between Rust and C++/CUDA.

## Rust Structures (ffi.rs)

All structures are marked `#[repr(C)]` for binary compatibility.

### `Environment`
Global simulation parameters.
```rust
pub struct Environment {
    pub global_oxygen_level: f32,  // 0.0 - 1.0
    pub dopamine_multiplier: f32,  // 1.0 baseline
    pub toxin_levels: f32,
    pub seed: u32,                 // Global chaos seed
}
```

### `BaseNeuron`
Main cell structure (must match `base_neuron.h`).
- `current_voltage`: Current potential (mV).
- `energy_ATP`: Current energy reserve.
- `metabolic_wear`: Mitochondrial wear (0.0 - 1.0).
- `is_alive`: Life flag.

## C++ Orchestration Functions

### `init_neuron`
Initializes a neuron based on type and heterogeneity.
```cpp
void init_neuron(BaseNeuron* neuron, uint32_t id, NeuronType type, uint32_t seed);
```
- Uses `seed + id` to generate unique Gaussian parameters.

### `launch_tick_neurons`
Main simulation cycle.
```cpp
void launch_tick_neurons(
    BaseNeuron* neurons, 
    int count, 
    Synapse* synapses, 
    int syn_count, 
    Environment env, 
    uint32_t current_tick
);
```
- Triggers metabolism and conduction calculations.
- On GPU, executes as two consecutive kernels with a barrier.

## CUDA Kernels

### `tick_neurons_kernel`
- Processes: `update_metabolism`, `update_calcium`, `process_spike`.
- Complexity: O(1) per thread.

### `tick_synapses_kernel`
- Processes: `process_synapse` (including STDP and delivery).
- Complexity: O(1) per thread.

## Rust Services (Orchestration)
Called from `main.rs` on the CPU side:
- `astrocyte_service`: Toxin cleanup, metabolic support.
- `microglia_service`: Apoptosis cleanup (removing dead neurons).
