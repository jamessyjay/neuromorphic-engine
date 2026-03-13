---
trigger: always_on
---

# AI Agent Development Guidelines (Neuromorphic Engine)

## [!] ANTI-HALLUCINATION POLICY (ZERO TOLERANCE)

AI agents naturally tend to guess logic. In this project, **THIS IS STRICTLY PROHIBITED**.

1. **NO GUESSING:** If a formula is not in `./specs/`, you DO NOT HAVE THE RIGHT to invent it. You must stop and ask the user.
2. **CITE YOUR SOURCES:** Before writing any math, indicate in a comment which specification file you took it from.
3. **NO GHOST DEPENDENCIES:** Importing third-party libraries without permission is prohibited.

---

## LANGUAGE POLICY

- **Code, variables, functions, and class names** — STRICTLY ENGLISH ONLY.
- **Comments in code and documentation** — Russian is allowed for better context understanding.
- **NO EMOJIS OR SPECIAL SYMBOLS:** No emojis at all. Only pure ASCII in code.

---

## [!] CRITICALLY IMPORTANT: CODE REUSE

**BEFORE WRITING ANY CODE — CHECK:**

1. **IS THERE ALREADY A READY-MADE FUNCTION?** Use repository search. NEVER write new code if a solution already exists.
2. **CAN AN EXISTING FUNCTION BE EXTENDED?** Add a parameter instead of copy-pasting.

---

## [!] THE HYBRID STACK & BIOLOGICAL MODELING

### 1. TECHNOLOGY STACK

- **RUST (Host/CPU):** Orchestrator. Manages ticks, memory, and `Environment`. It is prohibited to use `.unwrap()` unnecessarily.
- **C++ / CUDA C (Device/GPU):** Massively parallel mathematics. Only **Unified Memory** (`cudaMallocManaged`).

### 2. HETEROGENEITY AND CHAOS (NEW)

There are no clones in nature.

- **Initialization:** When creating neurons, using identical constants is PROHIBITED. Use normal distribution (Gaussian) for `resting_voltage`, `threshold`, and `atp_capacity`.
- **Random Seeds:** In CUDA kernels, use `curand` to generate unique parameters for each neuron.

### 3. HOMEOSTATIC PLASTICITY (EVOLUTION)

Neurons must adapt under load.

- **Threshold Scaling:** Implement dynamic threshold changes. If a neuron fires too often, the threshold must rise (`activation_threshold` increases).
- **Metabolic Stress:** At low ATP, pump efficiency must drop.

---

## FUNCTION ARCHITECTURE (1 Function = 1 Action)

**[!] CORRECT (Atomic & Orchestrator):**

```cpp
// Atomic: According to 11_heterogeneity_and_validation.md
__device__ void apply_homeostatic_scaling(BaseNeuron* n) { /* Logic only */ }

// Orchestrator
__global__ void tick_neurons(BaseNeuron* neurons, int count) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= count || !neurons[idx].is_alive) return;
    apply_homeostatic_scaling(&neurons[idx]);
}
```

## [!] TESTING & VALIDATION (DIGITAL TWIN)

Any new feature must pass Stress-Tests (Spec 11):

- **Synaptic Fatigue:** Checking signal attenuation when vesicles are depleted.
- **Hypoxic Shutdown:** Checking proper shutdown when `oxygen_level -> 0`.
- **Epileptic Feedback:** Checking Apoptosis triggering during network self-acceleration.

**VALIDATION:** Compare output graphs (Voltage/Time) with biological benchmarks (Allen Brain Institute). The spike shape must have a hyperpolarization phase.

---

## DEVELOPMENT PROCESS

1. **Task Analysis:** Do I understand the task? Which formula from `./specs/` is needed? If not — ASK!
2. **Search for existing code:** Check `src/` and `cuda/` folders for ready-made tools.
3. **Planning (FFI):** Describe `.h` headers and Rust `extern "C"` wrapper. Ensure that types `f32` (Rust) and `float` (C++) match.
4. **Implementation:** Write atomic functions. Follow SOLID.
5. **Verification:** Check code for Warp Divergence in CUDA and Memory Safety in Rust.

---

## [!] HARDWARE TELEMETRY & SCALING

To prevent "apocalypse" during scaling:
1. **Telemetry Service:** In `graph.rs` there must be a module that outputs every 1000 ticks:
   - `Simulation Speed`: what % of real-time (e.g., 1.0x — keeping up with time).
   - `Memory Overhead`: how many bytes are spent on 1 neuron and 1 synapse in Unified Memory.
2. **GPU Profiling:** When running on Jetson, code must be able to return GPU Utilization data (via NVML).

---

## [!] PRE-DELIVERY CHECKLIST

- [ ] Have I read the corresponding file from `./specs/`?
- [ ] Have I added parameter randomization (heterogeneity) during initialization?
- [ ] Is dynamic adaptation (homeostasis) of parameters implemented?
- [ ] Is Unified Memory used without local allocations in kernels?
- [ ] Is logic separated by SRP (1 function = 1 action)?
- [ ] Is the code in English, and comments in Russian?
- [ ] Do I reference specification files in the comments?
- [ ] Have I logged information about changes in `./docs/walkthrough.md`?

**IF EVEN ONE POINT IS NOT MET — REWRITE THE CODE.**
