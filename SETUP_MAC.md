# neuromorphic-engine: MacOS Local Environment Setup

Since CUDA cores and Nvidia's Unified Memory are not available on Mac (Apple Silicon / Intel) — features typical of Jetson Devices — we use a stub-based approach.

## 1. How the Code Works on Mac

When you run the project on macOS:

* The `build.rs` script detects that it's an Apple platform.
* It compiles the CUDA file `cuda/base_neuron.cu` as regular C++ code, using the system compiler (clang from Xcode).
* Macros like `__global__`, `__device__`, and other CUDA-specific qualifiers are hidden via the preprocessor: they are defined as empty (`#define __global__`).
* Instead of `cudaMallocManaged`, the default system `malloc` is used.
* The kernel logic (e.g., calculation of Leuk Rate or Calcium) is executed sequentially (in a for-loop) directly on the Mac's CPU.

This allows you to verify math, FFI operations, and orchestrator logic **without having a Jetson device on hand**.

## 2. Preparation

### **Step 1. Ensure Rust is installed**

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustc --version
```

### **Step 2. Install the C++ compiler (if not already present)**

```bash
xcode-select --install
```

### **Step 3. Installing Build Tools**

For C/C++ wrappers to work (`cc` in `build.rs`):
Usually, nothing additional is needed; `cc` will find `clang` on the Mac by itself.

## 3. Launching

Simply go to the `neuromorphic-engine` project folder and type:

```bash
cargo run
```

It will automatically compile `base_neuron.cu` into `librust_kernels.a`, link it with Rust, and output the test ticks to the terminal.

If the code crashes with FFI errors, ensure that the `BaseNeuron` structures in `cuda/base_neuron.h` match the `BaseNeuron` in `src/ffi.rs` 1:1 (order, size, and field types!).
