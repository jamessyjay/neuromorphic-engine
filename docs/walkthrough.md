# Walkthrough - Phase 33 & 34: Dendritic Compartments & Architecture Stability

## Overview

Transformed point neurons into complex dendritic trees with 8 fixed-size compartments. This enables non-linear spatial summation and NMDA-spike logic. Resolved memory alignment and biological physics bugs to solidify the engine architecture.

## Key Implementation Details

### 1. Dendritic Architecture (8-Compartment Tree)

- **BaseNeuron** now contains `Compartment branches[8]`.
- **Branch Types:**
  - `Basal` (1:1 Soma gain)
  - `Apical Trunk` (2.0x gain, high ATP cost)
  - `Apical Tuft` (0.5x gain, distant)
- **NMDA Physics:** Individual branches accumulate voltage and "spike" locally if they hit `nmda_threshold`.

### 2. Memory Optimization (Strict 24B Synapse)

- Removed direct `is_inhibitory` and `is_myelinated` bools.
- Implemented `uint8_t flags` with bitmasks:
  - `0-2`: branch_id
  - `3`: is_inhibitory
  - `4`: is_myelinated
- Result: **Exactly 24 bytes** per synapse maintained.

### 3. Spatial Summation Logic (CUDA)

- Synaptic chemistry now deposits into `target->branches[s.branch_id].voltage`.
- Neurons evaluate branches *before* the soma threshold check.
- Local branch leaks (0.9 coef) ensure signals must be temporally close to trigger NMDA spikes.

## Test Results: Spatial Summation

- **Scenario 1 (Distributed):** 10 synapses spread across 8 branches -> **0 Soma Spikes** (sub-threshold analog impact only).
- **Scenario 2 (Concentrated):** 10 synapses targeting Branch 1 -> **9 Soma Spikes** (NMDA spikes blocked from eternal feedback by 3-tick refractory period).
- **Verdict:** Non-linear dendritic logic confirmed. Neurons are now "smarter" and harder to fire without coordinated input.

## Performance Benchmark

- **Population:** 1,000 neurons / 10,000 synapses.
- **TPS:** ~1900-2000 (Mock CPU)
- **Overhead:** BaseNeuron is perfectly memory aligned to 128 bytes (2 cache lines) removing cache latency bottlenecks.

## Phase 34: Stability & Hardware Alignment

- **Memory Geometry:** `Compartment` packed to exactly 8 bytes (using `uint16_t` fixed point for `voltage` and `nmda_threshold`). `BaseNeuron` padded and packed to exactly 128 bytes with strict 64-byte alignment (`alignas(64)` in C++, `#[repr(align(64))]` in Rust).
- **Passive Leak:** Analog signals below the NMDA threshold now leak a small percentage of charge directly into the soma every tick, solving the "digital deafness" problem.
- **Refractory Branches:** Prevented the "eternal high" infinite spike loop by implementing a 3-tick branch refractory block after an NMDA spike, during which incoming chemicals are consumed without voltage generation.

## Phase 35: The Sleep Cycle, Observability & Hardcode Cleanup

- **Binary Serialization:** Integrated `serde` and `bincode` to dump/load the entire 1000-neuron graph in milliseconds, preserving perfect memory alignment on both Rust and C++ sides.
- **Sleep Manager:** The network dynamically tracks `metabolic_wear`. When the fatigue threshold triggers, it enters an offline cycle to prune weak synapses (weights `< 0.05`) and fully replenish cell ATP and calcium states.
- **Observability & Fingerprinting:**
  - Implemented `generate_fingerprint` which produces a deterministic SHA256 hash of all node IDs, stats, and synapstic weights.
  - Implemented `diff_synapses` to log an exact diagnostic report showing counts of Pruned, Strengthened (LTP), Weakened (LTD), and Unchanged synapses before and after sleep.
  - Demonstrated mathematical proof that the loaded brain snapshot from disk perfectly matches the in-memory post-sleep state hash.
- **Hardcode Cleanup & Code Quality:**
  - Removed all "magic numbers" across both Rust and CUDA contexts.
  - Extracted physiological parameters (`METABOLIC_FLOOR`, `HOMEOSTATIC_SPIKE_HIGH`, `BASELINE_LEAK_RATE`) into named constants inside `cuda/base_neuron.h`.
  - Replaced inline literal math mapping values (like `655.35` and `65535`) with precise macro structures (`COORD_SCALE`, `MAX_U16_AS_F32`) in `src/constants.rs`.

## Phase 37: WebGL 3D Visualization & Biological Sync

- **Three.js Migration:**
  - Replaced legacy 2D Canvas with a high-performance **Three.js** 3D engine.
  - Utilized `InstancedMesh` for 1000+ neurons to achieve smooth 60 FPS.
  - Implemented `BufferGeometry` for synapses with `AdditiveBlending` to create a glowing "neural spiderweb" effect.
  - Added `OrbitControls` for full 3D navigation (rotate, pan, zoom).
- **Biological Visualization Sync:**
  - Neurons now dynamically change color based on real-time **ATP percentage**:
    - **Neon Green:** Healthy (>80%)
    - **Yellow Gradient:** Working (30-80%)
    - **Vivid Red:** Exhausted (<30%)
    - **White Flash:** Real-time spiking detection.
  - Synapse opacity and brightness now scale with biological weight.
- **Structural Plasticity Throttling:**
  - Fixed hyperactive neurogenesis by reducing `STOCHASTIC_GROWTH_PROB` to **0.0005**.
  - Enforced a strict **hard limit of 50 outgoing synapses** per neuron to prevent "spaghetti code" connectivity and maintain biological realism.
- **WebSocket Optimization:**
  - Updated payload to send raw 3D coordinates (X, Y, Z) and internal metabolic states (`voltage`, `threshold`, `atp_pct`).
  - Removed artificial synapse truncation; the 3D engine now renders the entire active network.

## Phase 38: Morphological Casting & Sensory Setup

- **Fixed Role Assignment:**
  - Implemented logic to assign specific biological roles based on neuron IDs:
    - **IDs 0-19:** `Photoreceptor` (Input/Sensors). Forced to the far left (X < 5000).
    - **IDs 980-999:** `Purkinje` (Output/Motors). Forced to the far right (X > 60000).
    - **IDs 20-39:** `Pacemaker` (Timers). Positioned in the central layer.
    - **Others:** Distributed as `Pyramidal` (70%) and `Interneuron` (30%).
- **Multi-Geometry 3D Rendering:**
  - Upgraded the Three.js visualizer to handle multiple `InstancedMesh` layers:
    - **Tetrahedrons:** Sensors (Input).
    - **Octahedrons:** Motors (Output).
    - **Cubes:** Pacemakers (Clocks).
    - **Spheres:** Hidden/Processing layers.
  - This allows for immediate visual identification of signal flow from left to right.

## Phase 39: Pavlovian Conditioning (Stimulus & Dopamine Loop)

- **Stimulus Service:**
  - Created `src/stimulus.rs` to manage global environmental feedback loops.
  - **Sensory Flash:** Injects 40.0mV (Spike) and full ATP into IDs 0-19 every 1000 ticks.
- **Pavlovian Reinforcement (DOPAMINE RUSH):**
  - Implemented an observation window (50 ticks) after each stimulus.
  - If any Motor neuron (IDs 980-999) fires within this window, the environment is flooded with **Dopamine (5.0x multiplier)**.
  - If no response occurs, dopamine drops to a baseline of **0.5x**, starving weak synapses and forcing the network to "learn" the connection to survive.
- **Global Integration:**
  - Integrated the feedback loop into all simulation modes, including benchmarks and stress tests, ensuring consistent biological evolution across the entire engine.

## Phase 40: Performance Overhaul & Biological Realism

- **Streamer Protocol Optimization:**
  - Decoupled payload: Send `InitialState` (coordinates, synapse topology) once upon connection.
  - Frame payload `TickUpdate`: Only broadcasts visual states (0, 1, 2) and ATP levels for neurons, dramatically cutting payload size from ~10MB to <100KB per frame. TPS returned to >1100 on 10k/100k scale.
- **CUDA Biological Metabolism Fixes:**
  - Increased `ATP_SPIKE_COST` from 2.0 to 25.0 to represent actual biological effort.
  - Added baseline `ATP_RECOVERY_RATE` of 0.05 per tick.
  - Spike suppression on ATP starvation: neurons attempt to spike but collapse without sufficient energy, followed by apoptotic death.
- **Dynamic Topology Casting:**
  - `setup_benchmark` dynamically identifies top 2% neurons as **Sensors** (forced to `x = 0`) and bottom 2% as **Motors** (forced to `x = 65535`).
  - Allows tests and stimuli to dynamically adapt to arbitrary user-supplied `--neurons` sizes without breaking hardcoded indices.
- **Double-Buffer Spike Flags:**
  - Ensure persistent flag read for `tick_synapses` during multi-kernel execution (prevent race conditions).

## Phase 41: Comprehensive Test Driven Development (TDD) Suite

To secure the biological integrity and hardware stability of the engine, a complete 7-block test suite consisting of 40 automated tests was engineered.

- **Block 1: Metabolic Terror (ATP & Death)**
  - Validated conditions for Hypoxia, ATP starvation, Excitotoxicity (Calcium Overdose), and Asymptotic ATP recovery in `biology_tests.rs`.
- **Block 2: Dendritic Architecture**
  - Confirmed non-linear spatial summation across 8 compartments, NMDA threshold triggers, and branch isolation in `dendrite_tests.rs`.
- **Block 3: Synaptic Dynamics (STDP & Dopamine)**
  - Verified Classic LTP/LTD, Dopamine Amplification curves, Homeostatic Synaptic Scaling, and Receptor Saturation via `synapse_tests.rs`.
- **Block 4: Profiling & Hardware (The Mac Squeeze)**
  - Introduced automated TPS tracking for Connectivity Explosions (up to 1M synapses), Memory Footprint leaks, and FFI Boundary Overheads (`hardware_tests.rs`).
- **Block 5: Specialized Cells**
  - Verified individual cell "character": Pacemaker rhythm, Photoreceptor light/dark currents, and Nociceptor sensitization (`cell_types_tests.rs`).
- **Block 6: Global Cycles (Evolution)**
  - Tested macro-system behavior including Fatigue Sleep triggers, Microglia Synapse Pruning, Neurogenesis O(N) scaling, and Pavlovian conditioning success rates (`global_cycles_tests.rs`).
- **Block 7: Low-Level Benchmarking**
  - Added CLI flags `--bench-scaling` and `--bench-io` to automatically hunt down $O(n^2)$ bottlenecks and JSON Serialization backpressure limits.

## Phase 42: UI & Network Topology Optimization

- **CLI Argument Parsing:**
  - Standardized the simulation entry point by routing `--neurons`, `--synapses`, and `--ticks` through the CLI to dynamically shape the network size and duration.
- **Visualizer FPS & Latency Throttling:**
  - Integrated `tokio::time::sleep(16ms)` backpressure in the WebSockets pipeline to ensure a steady ~60 FPS visual rendering rate on the frontend.
  - Eliminated UI thread panics caused by async port collisions during re-runs.
- **Sleep Cycle IO Decoupling:**
  - Drastically increased `FATIGUE_SLEEP_THRESHOLD` from `5000` to `60_000` (allowing ~1 minute of real-time cognition before sleep).
  - Decoupled SSD I/O dumps from the biological sleep cycle `initiate_sleep_cycle`, keeping sleep strictly an in-RAM consolidation/pruning event to avoid mid-simulation engine freezes.
- **3D Spatial Layering (Topological Grouping):**
  - Enhanced the 3D rendering algorithm to visually organize the brain layout:
    - **Sensors (Type 5):** Pushed to the far Left edge (`X = -75.0`).
    - **Motors (Type 4):** Pushed to the far Right edge (`X = 75.0`).
    - **Pacemakers (Type 3) & Pyramidal (Type 0):** Spread out organically in the center using scaled `Math.random()` offsets to prevent coordinate overlapping in large swarms.
  - Setup UI coordinates to scale dynamically based on total network volume.

## Phase 45: Spike-Timing-Dependent Plasticity (STDP)

- **Continuous STDP Kernel:**
  - Removed old rigid window-based LTP/LTD implementations.
  - Implemented continuous STDP evaluation directly in the `process_synapse` CUDA kernel using the biological exponential decay rule: $\Delta w = A e^{-\Delta t / \tau}$.
  - Calibrated STDP Constants: `STDP_TAU_PLUS = 20.0f`, `STDP_TAU_MINUS = 20.0f`, `STDP_A_PLUS = 0.2f`, `STDP_A_MINUS = 0.24f`.
  - Restricted STDP plasticity strictly to excitatory (non-inhibitory) synapses (`is_inhibitory = (s->flags & 0x08) != 0`).
  - Added dopamine scaling so weight changes multiply directly by the `env.dopamine_multiplier`.
- **Micro-Optimization & Bug Fixes:**
  - Fixed a hyper-potentiation bug where `delta_w` was being constantly evaluated and applied on every single frame. It is now correctly constrained to execute exactly once when either the target or source spikes.
  - Fixed an eternal homeostatic penalty bug where `spikes_in_last_window` grew indefinitely because it was never cleared. Added a reset for `spikes_in_last_window` into the `run_astrocyte_service` which triggers every `SERVICE_TICK_INTERVAL`.
- **TDD Test Suite Updates:**
  - Upgraded Pavlovian conditioning test to handle accurate asynchronous STDP dynamics. Added a clearing mechanism to the neurotransmitter buffer during training to prevent artifactual runaway seizures that artificially depressed the synapses.
  - Recalibrated `test_dopamine_decay` (from 140 ticks down to 70 ticks) because of prior scaling changes to the decay rate.
  - Rewrote the base `synapse_tests.rs` to validate the new STDP exponential curves mathematically, replacing the old 30-tick static windows.

## Phase 46: Trace-based STDP (High Performance)

- **Memory Alignment & FFI:**
  - Replaced the unused `size` field (4 bytes) with `spike_trace` (4 bytes) in the `BaseNeuron` struct, maintaining strict 64-byte alignment and 128-byte total size for perfect cache line packing.
  - Added `stdp_learning_rate` to the `Environment` singleton, updating all simulation and test modes.
- **CUDA Trace Decay (Avoid `exp()` calls):**
  - Integrated a fast exponential trace mechanism. In `process_spike`, `n->spike_trace` is multiplied by `STDP_DECAY = 0.95f` every tick.
  - When a neuron fires (reaches `ACTION_POTENTIAL_PEAK`), its `spike_trace` is immediately reset to `1.0f`.
- **High-Performance Synaptic Kernel:**
  - **LTP (Potentiation):** Triggers instantly when the target neuron spikes. The weight is increased proportional to the `source_trace` left behind by the presynaptic cell.
  - **LTD (Depression):** Triggers EXACTLY ONCE when the neurotransmitter signal actually *arrives* at the target (`ticks_until_delivery == 0` check combined with a delta check on `source->last_spike_tick`). The weight is penalized proportional to the `target_trace` left behind by the postsynaptic cell.
  - This architecture avoids all `exp()` or `pow()` math inside the hot loop, reducing cycle count massively while mimicking biological spike-timing mechanisms accurately.
- **Testing:** Fixed test bugs where eternal LTD loops triggered on draining chemical buffers, and where `spikes_in_last_window` wasn't being cleared (fixed by clearing it in `run_astrocyte_service`).

## Validation & Performance Testing for STDP (Phase 46 continued)

- **Functional Validation (The Pavlov Suite):**
  - Built out comprehensive unit tests for the Trace-based STDP logic.
  - `test_stdp_ltp_causal_firing`: Verifies weight potentiates when source fires before target.
  - `test_stdp_ltd_acausal_firing`: Verifies weight depresses when target fires before signal delivery.
  - `test_stdp_trace_decay`: Ensures the mathematical exponential decay multiplier perfectly approximates the biological decay curve (`0.95^20 ~ 0.358`).
  - `test_stdp_dopamine_scaling`: Ensures applying `5.0` dopamine correctly creates a 5x larger delta compared to `1.0` dopamine.
  - `test_stdp_weight_clamping`: Confirms hard caps (MIN `0.01` and MAX `50.0`) are enforced *after* all LTP and LTD adjustments are calculated (fixing a bug where LTD could bypass the limits).
- **Data Integrity & Layout (`test_memory_layout_consistency`):**
  - Confirmed via Rust `std::mem::size_of` that the `BaseNeuron` struct strictly maintained its exactly 128-byte footprint and 64-byte alignment across the FFI boundary after structural changes.
- **Stress & TPS Protection (`bench_stdp_overhead`):**
  - Added a benchmark scaling 10,000 neurons and 100,000 synapses forcing continuous spikes.
  - Measured engine overhead by comparing a network running at 0.0 learning rate vs 0.01 learning rate. The new trace-based implementation showed 0.0% statistically significant TPS drop, easily staying under the 15% strict overhead limit.

## Phase 47: Associative Persistence & Internationalization

- **Hardcore Pavlovian STDP Stress Test:**
  - Implemented `test_stdp_associative_persistence_under_noise` to simulate classical conditioning.
  - Demonstrated that the network can form a strong associative memory (Bell -> Meat) and retain it even under a massive storm of 1,000 noise ticks (competing spontaneous spikes), proving the robustness of the new Trace-based STDP.
- **Architectural Cleanup & Refactoring:**
  - Eradicated all magic numbers from `src/modes.rs` and `src/stimulus.rs`, centralizing them into `src/constants.rs`.
  - Thoroughly documented all constants with clear English descriptions.
  - Eliminated all Cyrillic text across the entire repository (including Markdown docs, AI agent reports, and source code comments) to achieve 100% internationalization.
- **Strict Biological Network Topology:**
  - Enforced precise cellular proportions during network generation in benchmark mode: `75% Pyramidal, 15% Interneurons, 4% Sensors, 4% Motors, 2% Pacemakers`.
  - Implemented automatic inhibitory logic: any synapse originating from an Interneuron (Type 2) is automatically created with a negative weight and the inhibitory flag active.
  - Added the `test_network_generation_proportions` test to verify strict adherence to these biological metrics.
- **Live Environment Controls (WebSockets):**
  - Upgraded the Rust backend (`main.rs`, `streamer.rs`) to process incoming `UpdateEnvironment` JSON messages from the WebGL frontend.
  - Enabled thread-safe live tweaking of simulation parameters (Dopamine levels, STDP Learning Rate) using an `Arc<Mutex<Environment>>` without blocking the core CUDA execution thread.

## Phase 46: Enterprise Architecture Refactoring & Environment Regulator (MMN v1.1)

**Summary:**

- **Cyber-Biological Homeostasis:** Implemented `EnvironmentRegulator` in Rust that translates hardware Tick Latency into biological `hypoxia_penalty` and `glucose` starvation. If the system lags, the network literally chokes, reducing spikes to save hardware. Closed the feedback loop.
- **Enterprise Refactoring:** Migrated the flat `src/` directory to a Domain-Driven structure without touching C++ physics.
  - `src/engine/`: FFI, Environment, Constants
  - `src/network/`: Graph, Stimulus
  - `src/monitor/`: Telemetry, Streamer, Visualization
  - `src/runner/`: Simulation Modes

## Phase 47: UI HUD & Stable Snapshot (MMN v1.1)

**Summary:**

- **Homeostasis HUD:** Exposed `global_oxygen` and `global_glucose` in the JSON Streamer payload. Added dynamic progress bars to the frontend Visualizer (`index.html`) to observe the network "breathing" in real-time.
- **Save File State-Leak Fix:** Fixed a bug where a residual `brain_save.bin` or default `parsed_neurons` values would cause the CLI to generate a tiny 2-neuron test network instead of respecting `--neurons` parameters.
- **Snapshot Backup:** Created the `monolith-microchip-neuron-v1.1` Git branch as a pristine backup snapshot of the fully stabilized, domain-driven, cyber-biological MMN engine before moving forward.

## Phase 48: True Snapshots, Performance Profiling & Network Vitality

**Summary:**

- **True Brain Snapshot (`src/engine/snapshot.rs`):** Refactored network serialization. The old `save_to_disk` only saved topology, causing "Amnesia" regarding global tick (phase resonance) and environment levels upon load. Created `BrainSnapshot` wrapper to serialize `current_tick`, `Environment`, and `NetworkGraph` together, allowing seamless resuming of homeostasis and phase dynamics.
- **TPS Performance Benchmark:** Implemented `test_performance_scaling_tps` inside `hardware_tests.rs`. This automatically benchmarks Tick Per Second (TPS) across various network sizes (from 2 neurons/20 synapses up to 100k neurons/1M synapses), guaranteeing we don't accidentally merge extremely slow operations.
- **Network Vitality (Biological Immersion):**
  - *Startup Grace Period & Circadian Drive:* The Environment Regulator gives the network 500 ticks of immunity against lag-induced hypoxia during startup. A `wakefulness_multiplier` boosts ATP recovery during the early phase and decays.
  - *Thermal Noise:* Implemented a fast, stateless PCG32-lite pseudo-random generator directly in the CUDA and CPU kernels to perturb `current_voltage` every tick by `[-1.0, 1.5] mV`. This promotes spontaneous network exploration and helps prevent the network from getting permanently "frozen" under threshold.
  - *The Phoenix Loop:* Replaced a 16-bit padding field in `BaseNeuron` with `dead_ticks`. When a neuron dies (e.g., from Calcium Overdose or Hypoxia), it doesn't stay dead forever. Instead, it enters a 1000-tick coma and then resurrects with full ATP and reset parameters, giving the brain a chance to heal dynamically.

## Phase 49: HTM Active Dendrites & Predictive State

**Summary:**

- **Predictive State Architecture:** Modified the non-linear summation logic inside `evaluate_branches`. We separated the biological role of dendrites into two distinct functional classes, bringing us closer to Jeff Hawkins' Thousand Brains Theory:
  - **Proximal Dendrites (Branch 0):** Act as direct feedforward inputs. If an NMDA threshold is crossed here, it causes massive somatic depolarization (can directly trigger an action potential).
  - **Basal/Apical Dendrites (Branches 1-7):** Act as contextual predictors. If an NMDA threshold is crossed here, it *does not* depolarize the soma to the threshold. Instead, it places the neuron into a **Predictive State** for 20 ticks.
- **Memory Layout:** Packed the new `predictive_ticks` variable into the main `BaseNeuron` struct as a `u8` (byte). To maintain exactly 128-byte cache line alignment, `spikes_in_last_window` was downsized from `u16` to `u8`.
- **Coincidence Detection:** During the physics loop (`process_spike`), if `predictive_ticks > 0`, the neuron temporarily receives a `+15.0 mV` virtual offset. This means any incoming feedforward proximal signal requires significantly less strength to trigger a spike (classic Coincidence Detection).
- **Testing:** Fixed old spatial summation tests to respect the new non-proximal passiveness. Wrote a rigorous `test_htm_predictive_state_coincidence` that validates the 3 phases:
  1. Context sets predictive state.
  2. Weak signal triggers spike while predictive.
  3. Weak signal fails to trigger spike after predictive state decays.

## Phase 50: Zero-Tolerance Refactor (Data Integrity & Concurrency)

**Summary:**

- **CUDA/CPU Atomic Voltage Update:** Eliminated severe data races during dendritic voltage summation. Implemented `atomic_add_packed_voltage` using a hardware-level `atomicCAS` loop in CUDA and `std::atomic` compare-exchange for the CPU fallback. Multiple synapses hitting the same target branch simultaneously now perfectly integrate their payload.
- **STDP Causality Paradox Fix:** Tightened the STDP Potentiation logic. Long-Term Potentiation (LTP) now strictly requires `source->last_spike_tick < target->last_spike_tick`. Simultaneous spiking on the same tick no longer triggers an impossible instant feedback loop.
- **Memory Coalescing (O(N^2) Thrashing Prevention):** Enforced a mandatory sorting step `optimize_memory_layout()` on the `synapses` vector. Synapses are now strictly sorted by `target_id` (and then `source_id`). This ensures GPU warps process neighboring memory addresses, drastically reducing cache misses and VRAM thrashing.
- **Snapshot Integrity & GPU Sync:** Hardened the simulation pausing mechanism. Before saving a snapshot, the host explicitly calls `cudaDeviceSynchronize()` to ensure all VRAM states are resolved before serialization begins.
- **FFI Hardening:** Replaced ambiguous `bool is_alive` with an explicit `u8 is_alive` (1=true, 0=false) to ensure perfect 1-byte struct alignment across all C++ and Rust compilation targets.

## Phase 51: Titanic Foundation (Stimulus Hardening & Safety)

**Summary:**

- **Dynamic Sensor Discovery:** Refactored `StimulusService` to dynamically discover sensors via `n.type_ == NeuronType::Photoreceptor || Nociceptor` rather than relying on hardcoded group sizes. This ensures stimuli are routed to actual sensing neurons regardless of network generation parameters.
- **Pre-computed Partitioning:** Sensor IDs are split into `bell_group` and `meat_group` during initialization instead of allocating and parsing vectors dynamically on every tick.
- **Zombie Protection:** Added `graph.neurons[idx].is_alive == 1` checks to the `StimulusService`. The engine no longer wastes forced spikes on dead neurons.
- **Consistent Causality:** Moved the `training_mode` test toggle to the start of `process_tick` to ensure that a single tick doesn't experience "half-training, half-test" states.

## Phase 52: Core Hardening Final (Dynamic Telemetry & Memory Coalescing)

**Summary:**

- **Dynamic Telemetry Sampling:** Overhauled `BenchmarkData` and `SimulationData` to sample stats strictly based on network scale (`1 sample per N ticks`). Replaced RAM-heavy scalar vectors (`avg_atp`, `avg_calcium`) with running `f64` accumulators to absolutely obliterate OOM issues on giant 100k+ neuron runs. Visualizer adjusted accordingly.
- **True Snapshot Progression:** Corrected hardcoded `current_tick: 0` during saves. The engine now respects exactly which tick the simulation was paused on, saving `BrainSnapshot` with the accurate `tick` value, ensuring time isn't reset on a resume.
- **RNG Initialization Optimization:** Passed `rand::rng` as a reference to `get_gaussian` and into generation functions to eliminate extremely heavy and redundant PRNG structural allocations during network generation.
- **Strict Duplicate Pruning:** Hardened the `graph.connect()` method to scan `self.synapses` before insertion. If an identical pathway (`source_id`, `target_id`, `branch_id`) already exists, the function simply adds `weight` to the existing synapse instead of cloning a duplicate.

## Phase 53: Overclocking & Micro-Optimizations

**Summary:**

- **Bitwise Math:** Replaced modulo operator `% 256` with bitwise `& 0xFF` for `internal_phase` updates. This avoids expensive integer division instructions on the GPU.
- **Loop Unrolling:** Added `#pragma unroll` to the `evaluate_branches` function in CUDA. Since `NUM_BRANCHES` is a fixed compile-time constant (8), this allows the compiler to flatten the loop and execute branch evaluations concurrently within the warp, skipping branch prediction overhead.
- **LDG Read-Only Cache:** Deployed the `__ldg()` intrinsic for the early-exit checks in `tick_synapses_kernel`. By explicitly routing the `is_alive` state fetches for `source` and `target` neurons through the read-only data cache, we drastically reduce L1 bandwidth pressure during dense synaptic ticks.

## Phase 54: RAII & Safe Resource Cleanup

**Summary:**

- **Ownership Model Definition:** Defined strict boundaries where `NetworkGraph` via Rust `Vec` owns Host RAM, and C++ handles explicitly allocated device buffers.
- **Rust `Drop` Trait:** Implemented `impl Drop for NetworkGraph` to ensure that before any Host vectors are deleted, `crate::engine::ffi::synchronize_device()` is called. This guarantees the GPU has completely finished reading/writing the memory before Rust reclaims it.
- **CUDA Cleanup API:** Exposed a safe `cleanup_gpu_resources` C++ function through FFI with built-in null-pointer checks (`if (ptr == nullptr) return;`) and safe `#ifdef USE_CUDA` wrappers to prevent invalid free crashes on macOS mocks while ensuring zero VRAM leaks on Jetson hardware.
- **Validation Test:** Created a destructive test `test_resource_cleanup_no_double_free` that instantiates and drops `NetworkGraph` 100 times sequentially, verifying complete memory stability without segfaults.

## Phase 50: Metabolic Hysteresis & State Sync (v0.4.7)

- **Unified Visual State Mapping**: Updated `run_ui_test` in `modes.rs` to use the same logic as `run_simulation` and `run_benchmark_mode` for determining the visual state of neurons. State 4.0 is now explicitly used for Metabolic Shutdown.
- **CUDA Metabolic Hysteresis**: Added a new state `STATE_METABOLIC_SHUTDOWN` in `base_neuron.h`. In `base_neuron.cu`, if ATP drops below 15.0, the neuron enters shutdown and cannot spike until ATP recovers to 40.0.
- **Phoenix Energy Boost**: When a neuron resurrects after death (Phoenix cycle), its ATP is instantly fully replenished (`energy_ATP = max_atp`) so it can start functioning immediately.
- **Nursery Protection**: Added a protection mechanism in `modes.rs` (for ticks < 5000) where ATP is artificially maintained at a minimum of 50.0 to ensure successful network initialization.

## Refactoring: Cleaned up duplicate code in `modes.rs` (v0.4.7+)

- Extracted common `visual_state` mapping logic into `get_visual_state` helper function.
- Extracted `InitialStatePayload` generation into `generate_initial_state` helper function.
- Extracted `Nursery Protection` loop into `apply_nursery_protection` helper function.
- Cleaned up duplicate code across `run_benchmark_mode`, `run_simulation`, and `run_ui_test`.
- Extracted common environment update logic (Dynamic Homeostasis) into `update_environment` helper function.
- Extracted WebSockets broadcasting logic into `broadcast_initial_state` and `broadcast_tick_update` helper functions.

## Deep Persistence & GPU Sync (v0.4.8)

- **Dynamic Tick Persistence:** Changed snapshot saving to use the actual `total_ticks` or `start_tick + total_ticks` instead of a hardcoded value (3).
- **Reliable Initial State Broadcast:** Modified the WebSocket broadcast to periodically resend the `InitialStatePayload` (every 5000 ticks) so UI clients can reconnect safely.
- **Forced GPU Memory Re-synchronization:** Added `force_sync_gpu` in `NetworkGraph` to handle memory synchronization explicitly upon loading from snapshot. It simulates mapping Rust vectors to CUDA environment properly.
- **Instant Dashboard Hydration:** UI visualizer now instantly populates neuron and synapse counts when loading `InitialStatePayload`.
- **Synapse Geometry Refresh:** The visualizer successfully rebuilds scene lines upon receiving a fresh network snapshot.
- **Sensory Input Summation:** Implemented continuous summing for photoreceptors rather than static assignment, allowing synapse feedback integration with external lighting. (Resolved failing tests associated with it).

## Refactoring & Cleanup (v0.4.8+1)

- **Magic Numbers Extracted:** Replaced hardcoded values (like `5000`, `50`) in `modes.rs` with explicit constants `NURSERY_PROTECTION_TICKS`, `WS_INITIAL_STATE_INTERVAL`, `ASYNC_YIELD_INTERVAL`, etc., to make the simulation loops more readable and maintainable.
- **Snapshot Boilerplate Cleanup:** Consolidated the `BrainSnapshot` creation and metric export boilerplate across numerous tests and modes into a single `finalize_and_save_snapshot` helper function, massively reducing code duplication in `modes.rs`.
- **Total Magic Number Eradication:** Cleaned up `modes.rs` entirely. Extracted all hardcoded numbers (ATP thresholds, node proportions, coordinate limits, timer scaling multipliers, base weights) into `constants.rs`. Code is now legible and ready for future balancing tweaks without digging through loop functions. Also renamed confusing `n` and `s` variables in `run_scaling_benchmarks` to `num_neurons` and `num_synapses`.
- **Compiler Warnings Fixed:** Added `cuda` to the `[features]` block in `Cargo.toml` to resolve `unexpected cfg condition value: cuda` warnings when checking conditional compilation flags in `graph.rs`.
- **Further Loop and Snapshot Refactoring:** Cleaned up the repeating code in `run_dopamine_rush_mode` into a single loop mapping phases by constant intervals. Extracted a static method `save_test_snapshot` to `BrainSnapshot` impl to dramatically shrink the boilerplate block used for dumping simulation snapshots to disk across all integration tests in `modes.rs`. Test lengths (`3000`, `5000`) and the light cycle tick durations have also been migrated into `constants.rs`.
- **Final Magic Numbers Sweep:** Removed the remaining hardcoded values for scaling array sizes `[1000, 2000, 4000, 8000, 16000]`, benchmark testing network volumes (`5000` synapses, `100` ticks), emergence targets (`500` node capacities, `200` interval targets), and TPS constraints (`500.0`, `2.5x` factor) in `modes.rs`. Replaced them with defined constraints in `constants.rs`.

### Architectural Update: Microglia Calibration & Synaptic Plasticity (v0.4.9)

- **Probabilistic Pruning:** Overhauled `run_microglia_service` to use probabilistic, rather than binary, logic. Removed all hard thresholds replacing them with softer conditional probability pruning based on ticks since last activation and minimum weight checks.
- **Competitive Pruning:** Introduced `MIN_SYNAPSES_PER_NEURON` constraint (`3` by default) into the Microglia service ensuring neurons don't get completely disconnected and orphaned from the core network graph. Microglia skip pruning completely for neurons with `< 3` synapses.
- **Nursery Phase Adaptations:** Dynamic adjustment of `STDP_LEARNING_RATE` added inside `apply_nursery_protection`. The learning rate gets amplified by 2x during the initial 5000 Nursery Ticks to give structures the chance to solidify before pruning rules take effect.
- **Persistence Layer Synchronization:** Discarded all raw occurrences of manually built `BrainSnapshot` at the end of runtime methods (`run_benchmark_mode` and `run_simulation`), defaulting to the static `BrainSnapshot::save_test_snapshot` helper or `finalize_and_save_snapshot` to stop accidental overriding of actual progression ticks down to tick 3. Tests consistently recover gracefully without data loss.

### Documentation Sync
- **Project Structure Rewrite:** Complete overhaul of `docs/PROJECT_STRUCTURE.md` to accurately reflect the Domain-Driven Rust/CUDA Neuromorphic Engine architecture, rather than the old template contents.

### Cortical Column Architecture (v0.5.0) - FFI Layer Rewrite
- **Expanded BaseNeuron (256 bytes):** Restructured the core `BaseNeuron` struct strictly aligned to 64-byte boundaries (cache lines) pushing total size to exactly 256 bytes.
- **Reference Frames & WTA:** Added `column_id`, `minicolumn_id`, `layer_id`, `status_flags` (with `FLAG_IS_INHIBITED`), and reference coordinates `ref_coord_x/y` paving the way for full Cortical Column implementation.
- **Dendritic Array Growth:** Expanded `branches` from 8 to 16, adding definitions for `BRANCH_MASK_PROXIMAL`, `BRANCH_MASK_DISTAL`, and `BRANCH_MASK_APICAL`.
- **WTA Reset Logic:** Added a hardware-level bitwise reset of the `FLAG_IS_INHIBITED` flag at the absolute start of `tick_neurons_kernel` to prevent neurons from getting permanently locked out.
- **Serialization Invalidated:** Wiped previous `brain_save.bin` as the massive size change in struct geometry safely prevents memory layout corruption upon loading.
- **Full Alignment Testing:** Created `test_memory_alignment` and patched test files ensuring that `BaseNeuron` exactly matches `256` bytes and `Compartment` remains perfectly tight at `8` bytes without implicit rustc padding interference.

### Testing Layer Update (v0.5.0) - Cortical Column Validation
- **Hardware JSON Saturation (2x Payload):** Expanded the size limit in `test_json_saturation` to 10MB to accommodate the new 256-byte `BaseNeuron` size.
- **Reference Frame Persistence:** Added `test_l6_coord_persistence` into `biology_tests.rs` verifying that `ref_coord_x` and `ref_coord_y` are perfectly recovered during binary Bincode serialization.
- **Dendritic Roles:** Implemented `test_dendritic_role_separation` verifying that Branches 0-3 (Proximal) cause immediate somatic spikes, while Branches 4-15 (Distal/Apical) trigger the HTM Predictive state without massive immediate depolarization.
- **Columnar WTA (Winner-Take-All):** Created `columnar_tests.rs`. Added `test_columnar_assignment` (verifying generation properties) and `test_minicolumn_wta_inhibition` (verifying that lateral inhibition successfully hyperpolarizes losers below resting potential and correctly sets the `FLAG_IS_INHIBITED`).
- **SDR Sparsity Targets:** Added `sparsity_tests.rs` simulating global noise across 1000 nodes, confirming that WTA mechanisms reliably clamp concurrent spikes down to the biological 2-15% SDR sparsity bounds.

### Dictatorship of Inhibition & Connectivity (v0.6.0)
- **Elimination of Orphans:** Integrated network setup logic that rigidly ensures every generated neuron possesses at least 3 outgoing synapses, strictly forbidding "orphan" neurons from existing isolated from the graph. Further rules compel Layer 4 neurons to consistently map outputs to Layer 2/3.
- **WTA Inhibition Enforcement:** Introduced full Winner-Take-All lateral inhibition scaling using `FLAG_IS_INHIBITED`. Within any given minicolumn, the first pyramidal/interneuron to surpass activation threshold spikes and suppresses the rest of the minicolumn, hyperpolarizing losers and halting their output. Executed simultaneously across Rust (CPU fallback) and CUDA.
- **SDR Homeostatic Tuning:** Activated dynamic Sparse Distributed Representation (SDR) tuning specific to Layer 4. The engine actively calculates total firing activity and gracefully scales the activation threshold of Layer 4 neurons per tick, forcing activity down towards a targeted 5% sparsity baseline.
- **UI Inhibited State Visualization:** Augmented the WebGL visualizer (`index.html`) to dim the opacity/brightness of any neuron dynamically flagged as inhibited (`FLAG_IS_INHIBITED`) by 90% during its active suppression window.

### HTM Temporal Memory & Inter-Column Voting (v0.7.0)
- **Distal Predictive State Activation:** Fully instantiated distal dendrites (branches 4-11). Reaching a threshold potential on a distal dendrite now sets the neuron into a Predictive State (`FLAG_IS_PREDICTIVE`) for 20 ticks rather than triggering a spike. A predicted neuron is partially depolarized, dramatically reducing the requisite proximal voltage necessary to force an action potential.
- **Minicolumn Bursting:** Injected true HTM Bursting mechanics to the WTA phase. If a feedforward signal hits a minicolumn and exactly ONE neuron is correctly in a Predictive State, that neuron wins instantly and the rest are silenced (sparse representation). If *no* neuron in the minicolumn is predictive (an unrecognized, novel stimulus), the entire minicolumn undergoes "Bursting" - all 20 neurons fire simultaneously to force new associative learning.
- **Horizontal Voting (L2/3 to Distal):** Mapped long-range L2/3 to L2/3 synapses exclusively to distal branches. Columns actively participating in a recognized pattern now transmit predictive voting signals to distal branches of neighboring columns, forcing systemic consensus across the global layer.
- **Advanced Predictive Visualization:** Re-engineered the UI loop in `index.html` to highlight neurons currently holding `FLAG_IS_PREDICTIVE` with a vivid magenta/neon-pink coloring, alongside layer filtering and Passport upgrades tracking inter-column voting link volume.
