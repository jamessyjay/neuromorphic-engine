---
trigger: always_on
---

# AI Agent Development Guidelines (Neuromorphic Engine)

Welcome to the Neuromorphic Engine. This project is a biologically plausible, massively parallel neural network simulation designed to run on the GPU (CUDA/C++) while being orchestrated by a high-performance host (Rust). It features Homeostatic Plasticity, Dendritic computation, STDP, and dynamic Environment regulation.

Before touching anything, READ THIS CAREFULLY.

## [!] ANTI-HALLUCINATION POLICY (ZERO TOLERANCE)

AI agents naturally tend to guess logic. In this project, **THIS IS STRICTLY PROHIBITED**.

1. **NO GUESSING:** If a formula is not in `./specs/`, you DO NOT HAVE THE RIGHT to invent it. You must stop and ask the user.
2. **CITE YOUR SOURCES:** Before writing any math, indicate in a comment which specification file you took it from.
3. **NO GHOST DEPENDENCIES:** Importing third-party libraries without permission is prohibited.

---

## [!] DOMAIN-DRIVEN ARCHITECTURE (RUST)

The project uses a strict domain-driven module structure. Do not place everything in the root `src/` directory.

- `src/engine/`: Core logic, FFI bindings (`ffi.rs`), Constants (`constants.rs`), and Environment Regulation (`environment.rs`).
- `src/network/`: The brain graph topology (`graph.rs`) and external stimulus injection (`stimulus.rs`).
- `src/monitor/`: Live WebSocket UI streamers (`streamer.rs`), Data exporters (`visualization.rs`), and Telemetry (`telemetry.rs`).
- `src/runner/`: Simulation loops, execution modes, and benchmark handlers (`modes.rs`).

*Note: All C++ and CUDA code remains in the `cuda/` directory.*

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

### 2. HETEROGENEITY AND CHAOS

There are no clones in nature.

- **Initialization:** When creating neurons, using identical constants is PROHIBITED. Use normal distribution (Gaussian) for parameters.
- **Random Seeds:** In CUDA kernels, use `curand` to generate unique parameters for each neuron.

### 3. HOMEOSTATIC PLASTICITY (EVOLUTION)

Neurons must adapt under load.

- **ATP-Gating & Threshold Scaling:** If a neuron fires too often or lacks ATP, the dynamic threshold must rise (`activation_threshold` increases), ignoring weak stimuli.
- **Metabolic Stress:** Handled dynamically via Rust's `EnvironmentRegulator`, which drops Global Oxygen/Glucose if the hardware lags.

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

Any new feature must pass the rigorous validation suites in `tests/`:

- **Synaptic Fatigue & Receptor Saturation:** Validated in `synapse_tests.rs`.
- **Hypoxic Shutdown:** Handled globally by `EnvironmentRegulator` and tested in `biology_tests.rs`.
- **Dendritic Logic (NMDA AND-Gates):** Validated in `neuron_physics_tests.rs`.
- **Precise STDP & Pavlovian Learning:** Tested with causal/acausal firing in `test_stdp.rs`.

**VALIDATION RULE:** All new Rust logic must have a passing unit test before committing. Never merge failing tests.

---

## DEVELOPMENT PROCESS

1. **Task Analysis:** Do I understand the task? Which formula from `./specs/` is needed? If not — ASK!
2. **Search for existing code:** Check `src/` (in proper subdirectories) and `cuda/` folders for ready-made tools.
3. **Planning (FFI):** Ensure that types `f32` (Rust) and `float` (C++) match exactly. Be hyper-aware of `#[repr(C)]` alignment!
4. **Create tasks checklist** Create checklist with a list of tasks whicb are about to be executed. After every completed task mark the task in the checklist as completed. Review the checklist before proceeding with the next task (maybe it is already done)
5. **Implementation:** Write atomic functions. Follow SOLID.
6. **Verification:** Run `cargo test`. Ensure 100% tests pass.

---

## [!] PRE-DELIVERY CHECKLIST

- [ ] Have I read the corresponding file from `./specs/`?
- [ ] Is Unified Memory used without local allocations in kernels?
- [ ] Did I place the Rust file in the correct Domain directory (engine, network, monitor, runner)?
- [ ] Is logic separated by SRP (1 function = 1 action)?
- [ ] Are all 60 tests still green? (`cargo test`)
- [ ] Is the code in English, and comments in Russian?
- [ ] Have I logged information about changes in `./docs/walkthrough.md`?

**IF EVEN ONE POINT IS NOT MET — REWRITE THE CODE.**
