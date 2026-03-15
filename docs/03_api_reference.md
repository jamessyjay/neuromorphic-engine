# Neuromorphic Engine: API & FFI Reference

Technical description of the interaction interfaces between the Rust Orchestrator and the C++/CUDA Device.

## Rust Structures (`ffi.rs`)

All structures are explicitly marked `#[repr(C)]` to guarantee strict byte-level alignment between Rust and C++ across the FFI boundary.

### `Environment`

Global simulation parameters passed continuously to the device on every tick.

```rust
#[repr(C)]
pub struct Environment {
    pub global_oxygen_level: f32,    // 0.0 - 1.0 (Affected by hardware lag)
    pub global_glucose_level: f32,   // 0.0 - 1.0 (Metabolic floor)
    pub dopamine_multiplier: f32,    // 1.0 baseline (Scales STDP)
    pub toxin_levels: f32,           // Brain waste
    pub light_level: f32,            // Exogenous input for Photoreceptors
    pub stdp_learning_rate: f32,     // Base LTP/LTD scale (Boosted in Nursery)
    pub wakefulness_multiplier: f32, // Circadian rhythm influence on ATP
}
```

### `BaseNeuron`

Main cell structure. Its layout is completely identical to `base_neuron.h`.

- `current_voltage`: Current membrane potential (mV).
- `energy_ATP`: Current energy reserve (0.0 to 100.0).
- `is_alive`: 1 if functioning, 0 if dead.
- `dead_ticks`: Counter for the Phoenix Loop coma state.
- `predictive_ticks`: Countdown for HTM Predictive State (Coincidence Detection).

## Unified Memory Orchestration

The engine relies on `cudaMallocManaged` through custom C++ allocators to ensure memory is accessible simultaneously by Rust and CUDA without explicit copying.

### `allocate_neurons` / `allocate_synapses`

```cpp
BaseNeuron* allocate_neurons(size_t count);
Synapse* allocate_synapses(size_t count);
```
- Allocates contiguous blocks of memory mapped into the Unified Memory space. 

### `cleanup_gpu_resources`

```cpp
void cleanup_gpu_resources(BaseNeuron* neurons, Synapse* synapses);
```
- Called automatically by the Rust `NetworkGraph` via the `Drop` trait. Guarantees `cudaDeviceSynchronize()` is awaited before issuing `cudaFree()` to prevent VRAM leaks.

## C++ Orchestration Functions

### `init_neuron`

Initializes a neuron based on its biological type, employing Gaussian distribution for natural heterogeneity.

```cpp
void init_neuron(BaseNeuron* neuron, uint32_t id, NeuronType type);
```
- Internally uses `curand` or PCG32-lite to seed the noise profile.

### `launch_tick_neurons`

The primary execution entry point triggered every millisecond (Tick) by Rust.

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
- Dispatches computation to the GPU. Executes as two consecutive kernels (`tick_neurons_kernel` and `tick_synapses_kernel`) separated by an implicit grid synchronization barrier.

## CUDA Kernels (`base_neuron.cu`)

### `tick_neurons_kernel`
- Processes: Metabolism (`update_metabolism`), thermal noise, calcium toxicity checks, threshold crossing (`process_spike`), and HTM Predictive State decays.
- Parallelization: O(1) per neuron thread.

### `tick_synapses_kernel`
- Processes: Neurotransmitter delivery (`process_synapse`), receptor saturation, dendritic evaluation (`evaluate_branches`), and causal STDP weight modification.
- Parallelization: O(1) per synapse thread.
- Memory: Employs `__ldg()` for reading read-only neuron states (like `is_alive`) via texture cache to save L1 bandwidth.
- Concurrency: Utilizes `atomicCAS` loops to safely integrate multiple voltages hitting the same dendritic branch concurrently.

## Rust Background Services (`graph.rs`)

Called from `modes.rs` on the CPU side during execution loops:

- `run_astrocyte_service(tick)`: Replenishes ATP reserves, flushes toxins, and restores neurotransmitter pools based on `wakefulness`. Runs every 10 ticks.
- `run_microglia_service(tick)`: Probabilistic and competitive pruning of weak/inactive synapses. Respects the `MIN_SYNAPSES_PER_NEURON` floor. Runs every 500 ticks.
- `run_neurogenesis_service(tick)`: Stochastically sprouts new synapses between active, proximal neurons with high ATP. Runs every 1000 ticks.
