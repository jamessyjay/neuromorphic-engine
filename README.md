# Neuromorphic Engine

**Neuromorphic Engine** is a high-performance, biologically inspired neuromorphic simulation engine. It implements a hybrid architecture combining **Rust** for high-level orchestration and memory safety with **C++/CUDA** for massively parallel biological computations.

The project is designed for deployment on edge AI hardware like **NVIDIA Jetson**, utilizing **Unified Memory** for efficient data sharing between CPU and GPU.

---

## Detailed Documentation

For a deep dive into the architecture and logic of the project, visit the [docs/](file:///Users/dmitry.ledenyov/Documents/Personal/Self_Development/AI/neuromorphic-engine/docs/) folder:

- 🏗️ [Project Architecture](file:///Users/dmitry.ledenyov/Documents/Personal/Self_Development/AI/neuromorphic-engine/docs/01_project_architecture.md) — Hybrid Rust/CUDA stack and memory management.
- 🧬 [Solution Architecture](file:///Users/dmitry.ledenyov/Documents/Personal/Self_Development/AI/neuromorphic-engine/docs/02_solution_architecture.md) — Biological algorithms, chaos, and metabolism.
- ⚗️ [API & FFI Reference](file:///Users/dmitry.ledenyov/Documents/Personal/Self_Development/AI/neuromorphic-engine/docs/03_api_reference.md) — Bridges between Rust and C++.
- 📊 [Visualization & Metrics](file:///Users/dmitry.ledenyov/Documents/Personal/Self_Development/AI/neuromorphic-engine/docs/04_metrics_and_viz.md) — Guide to tests and result analysis.

---

## 🛠 Features Breakdown

### Biological Mechanisms

- **Leaky Integrate-and-Fire (+):** Advanced neuron models with metabolic constraints.
- **Metabolism Simulation:** Neurons consume **ATP** and generate **Calcium**; critical levels lead to apoptosis (programmed cell death).
- **Hebbian Learning (STDP):** Synapses strengthen/weaken based on precise spike timing (20ms window) and global **Dopamine** levels.
- **Neurotransmitter Dynamics:** Detailed modeling of Glutamate/GABA spikes, receptor saturation, and astrocyte-mediated degradation.

### Specialized Cell Types

- **Sensory Cells:** `Photoreceptor` (analog signal processing) and `Nociceptor` (pain sensitization).
- **Structural Cells:** `SchwannCells` (myelination) providing zero-delay signal transmission.
- **Circuit Diversity:** `Pyramidal`, `Bipolar`, `Interneuron`, and `Purkinje` cells with unique activation profiles.

### Orchestration & Services

- **Astrocyte service:** Manages synaptic health and ATP distribution.
- **Microglia service:** Automated pruning of dead connections and garbage collection.
- **Environment:** Global simulation parameters (Oxygen, Dopamine, Toxins).

---

## Architecture

- **Rust Subsystem:** Handles the `NetworkGraph`, FFI bindings, background services, and native data visualization via `plotters`.
- **Parallel Processing (Phase 12):** Multi-kernel architecture for maximum GPU throughput:
  - `tick_neurons_kernel`: Independent biology (metabolism, spiking) per neuron.
  - `tick_synapses_kernel`: Massive synapse-parallel processing (100k+ connections) using **atomic operations** (`atomicAdd`) for thread-safe voltage updates.
- **CUDA Subsystem:** High-speed kernels for voltage integration, synaptic processing, and metabolic updates.
- **FFI Layer:** Zero-copy interaction using raw pointers and C-compatible structs.

---

## Performance & Benchmarking

The engine includes a dedicated benchmark mode for stress-testing large-scale networks.

**Baseline Metrics (macOS Mock-CPU):**

- **Network Size:** 10,000 Neurons / 100,000 Synapses.
- **Ticks Per Second (TPS):** **~6,150 Ticks/sec**.
- **Efficiency:** Processing a single millisecond tick in ~0.16ms.

---

## How to Run

### 1. Requirements

- Rust (Cargo)
- C++ Compiler (Clang/GCC)
- *Optional:* CUDA Toolkit (for Jetson deployment)

### 2. Run Simulation (Visual Mode)

Generates activity charts of a test circuit in `graphs/phase8_sensors.png`.

```bash
cargo run --release
```

### 3. Run Benchmark (Performance Mode)

Executes a load test on 10k neurons and generates `graphs/benchmark_load.png`.

```bash
cargo run --release -- --benchmark
```

---

## Directory Structure

- `src/`: Rust source code (App orchestrator, FFI, Graph management).
- `cuda/`: C++/CUDA kernels for biological simulation.
- `specs/`: Detailed technical specifications and mathematical models.
- `graphs/`: Auto-generated visualization results.

---
*Created with ❤️ by the DL.*
