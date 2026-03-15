# **Project Overview: Neuromorphic Engine**

This document provides a technical summary of the Neuromorphic Engine repository, its structure, implemented features, and technical specifications.

## **1. Repository Structure & Key Locations**

The project follows a strict domain-driven modular architecture designed to cleanly separate the high-performance CPU orchestrator from the massively parallel GPU physics engine.

* **`src/` (Rust Host / CPU)**: Orchestrator. Manages ticks, memory, and environment constraints.
  * **`engine/`**: Core logic, FFI bindings to C++ (`ffi.rs`), global constants (`constants.rs`), and dynamic environment regulation (`environment.rs`, `snapshot.rs`).
  * **`network/`**: The brain graph topology (`graph.rs`), neurogenesis/microglia services, and external stimulus injection (`stimulus.rs`).
  * **`monitor/`**: Live WebSocket UI streamers (`streamer.rs`), data exporters (`visualization.rs`), and system telemetry (`telemetry.rs`).
  * **`runner/`**: Simulation loops, execution modes, and automatic benchmark handlers (`modes.rs`).
* **`cuda/` (C++ / CUDA C Device)**: Massively parallel mathematics. Contains kernel definitions (`base_neuron.cu`, `base_neuron.h`) utilizing Unified Memory (`cudaMallocManaged`).
* **`tests/`**: Rigorous validation suites acting as a "Digital Twin". Includes testing for synaptic fatigue, hypoxic shutdown, precise STDP, dendritic logic, and hardware bottlenecks.
* **`specs/` & `docs/`**: Declarative definitions for biological models, hardware targets, engine limits, and historical walkthroughs.
* **`visualizer/`**: Vanilla HTML/JS frontend (`index.html`) providing a real-time WebSocket HUD for the brain.

## **2. Implementation Status**

| Feature | Location | Status |
| :--- | :--- | :--- |
| **GPU Massively Parallel Compute** | `cuda/base_neuron.cu` | Implemented (Unified Memory + Atomic Operations) |
| **Homeostatic Plasticity (ATP/O2)** | `src/engine/environment.rs` | Fully implemented (Dynamic threshold scaling) |
| **STDP & Dopamine Modulation** | `src/network/stimulus.rs` / `cuda/` | Implemented (Precise Pavlovian logic) |
| **HTM Active Dendrites** | `cuda/base_neuron.cu` | Implemented (Predictive states & NMDA multiplexing) |
| **Neurogenesis (Growth)** | `src/network/graph.rs` | Implemented (Stochastic spatial sampling) |
| **Microglia (Synaptic Pruning)** | `src/network/graph.rs` | Implemented (Probabilistic & Competitive) |
| **True Persistence (Snapshots)** | `src/engine/snapshot.rs` | Implemented (Full state & telemetry recovery) |
| **Live WebSockets HUD** | `src/monitor/streamer.rs` | Operational for UI & 3D WebGL renderers |

## **3. Project Specifications**

* **Architecture**: Hybrid CPU/GPU stack. Rust acts as the orchestrator; CUDA handles the physics matrix.
* **Memory Model**: Strictly Unified Memory (`cudaMallocManaged`) for zero-copy synchronization between Host and Device.
* **Biological Time**: 1 Tick = 1 ms of biological time.
* **Structural Limits**: Supports massive scale (up to 100k+ neurons and 1,000,000+ synapses). 8 dendritic branches max per Pyramidal neuron.
* **Heterogeneity**: No "clones". All initializations use normal distributions (Gaussian) for parameters. Device-side randomness powered by `curand`.
* **Numerical Format**: 32-bit Floating Point (`f32` in Rust, `float` in C++) perfectly aligned via `#[repr(C)]`.

## **4. Key Functionalities**

1. **Hardware-Environment Feedback Loop**: `EnvironmentRegulator` monitors hardware latency (TPS) and maps it to biological "hypoxia". If the host system lags, global oxygen drops, slowing down neuron firing to save the hardware.
2. **Biologically Plausible Physics**: Complete simulation of ATP metabolic cycles, Oxygen depletion, Calcium toxicity, Sleep cycles, and Synaptic fatigue.
3. **Continuous Lifelong Learning**: Dynamic topology through neurogenesis (growth) and microglia (pruning), combined with STDP (Spike-Timing-Dependent Plasticity) for associative learning.
4. **Predictive States (HTM)**: Dendritic multiplexing distinguishes direct proximal excitation from contextual basal/apical predictive states (Thousand Brains Theory).

## **5. Dependencies & Requirements**

### **Rust Backends (Host)**
* **tokio / warp**: Asynchronous runtime and WebSockets for the UI HUD.
* **rayon**: Work-stealing parallelism for CPU-bound background services (Astrocytes, Telemetry).
* **plotters**: Data visualization and metric chart rendering.
* **serde / bincode**: High-speed binary snapshot serialization and JSON exports.
* **rand**: High-entropy stochastic seed generation.

### **C++ / CUDA Backend (Device)**
* **CUDA Toolkit (v11+)**: Required for `nvcc` compilation and GPU acceleration.
* **curand**: Hardware-accelerated device-side random number generation.
* **C++ Compiler**: Clang/GCC for FFI integration during the `build.rs` phase.

### **Frontend**
* **Vanilla HTML5 / JS**: Minimalist visualization (`visualizer/index.html`) using standard WebSocket API.
