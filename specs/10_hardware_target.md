# Hardware Target & Hybrid Tech Stack

## 1. Target Platform
* **Hardware:** NVIDIA Jetson Orin Nano (8GB RAM).
* **Features:** 1024 CUDA cores (Ampere), 6-core ARM Cortex-A78AE CPU, **Unified Memory** (Shared memory between CPU and GPU, eliminating the need to copy data back and forth).

## 2. Hybrid Stack (Rust + C++/CUDA)
The project uses a hybrid architecture to achieve maximum safety and speed.
* **Rust (Host / CPU):** Responsible for the `Environment` class, tick counting (Tick System), networking, logging, and program lifecycle management.
* **C++ / CUDA C (Device / GPU):** Responsible EXCLUSIVELY for the parallel mathematics of neurons (Leak, Calcium, Spike, Weights update).

## 3. Interaction Architecture (FFI)

* **Unified Memory:** The array of states for all neurons (charges, timers, chemistry) is allocated in Rust using CUDA Unified Memory (`cudaMallocManaged` via FFI). Both languages look at the same chunk of physical memory.
* **Calling Kernels:** On each tick, Rust calls a C++ function (via `extern "C"`), which launches a `__global__` kernel on the GPU for parallel recalculation of all 1024+ neurons.

## 4. Agent Task (CRITICALLY IMPORTANT)
The lead developer (human) focuses on the architecture in Rust.
Your task (as the AI agent) is to take on all the heavy lifting of writing the **C++/CUDA code**.
1. You must provide ready-made `.cu` files containing the neuron logic.
2. You must provide C-header files (`.h`) for the bridge.
3. You must generate Rust wrappers (via `bindgen` or manual `extern "C"`) and `build.rs` scripts so that Rust can seamlessly compile and call your CUDA code.