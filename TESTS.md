# Neuromorphic Engine - Comprehensive Testing Suite

This document outlines the testing strategy for the Neuromorphic Engine. It is divided into 7 blocks, covering everything from basic biological accuracy to low-level hardware performance and non-linear scaling detection.

**IMPORTANT NOTE**
All tests should be run with `cargo test --release` to ensure they are testing the optimized code.
Also, for pure results there must be no other heavy processes running on the system. Ensure that CPU load is less than 5% before starting the testing.

## Block 1: Metabolic Terror (ATP & Death)

*Goal: Ensure `base_neuron.cu` correctly calculates the price of life.*

- **1. Test "Metabolic Sink":** Creates 100 neurons, applies continuous maximum stimulus. Verifies that neurons rapidly consume ATP and die via hypoxia (`DEATH_ATP_LOW`).
- **2. Test "ATP Recovery Asymptote":** Measures ATP recovery speed for an empty neuron. Confirms the logarithmic recovery curve (fast when empty, slow when near full) driven by `atp_gap / (max_atp * 0.5f)`.
- **3. Test "Mitochondrial Failure":** Sets `global_glucose_level` to 0.1. Verifies neurons slowly die from starvation even with minimal activity.
- **4. Test "Metabolic Floor Leak":** Leaves network idle for 10,000 ticks. Verifies `BASELINE_LEAK_RATE` does not kill neurons but keeps them stable at resting levels.
- **5. Test "Oxygen Deprivation":** Sets `global_oxygen_level` to 0.0. Measures time until mass die-off due to severe hypoxic penalty.
- **6. Test "Calcium Overdose":** Forces extreme firing rate without refractory periods. Verifies neuron triggers `DEATH_CALCIUM_HIGH` (excitotoxicity).
- **7. Test "Calcium Recovery Rate":** Induces a single spike and verifies calcium pumps effectively return the level to 0 over time.

## Block 2: Dendritic Architecture (Compartments & Summation)

*Goal: Verify non-linear dendritic branch logic.*

- **8. Test "Spatial Summation (Basal)":** Supplies weak signals to a single basal branch over time. Verifies the integration successfully depolarizes the soma.
- **9. Test "NMDA Spike Trigger":** Pushes a branch voltage past the `nmda_threshold`. Verifies the massive `NMDA_SPIKE_GAIN` (5.0x) is injected into the soma.
- **10. Test "Branch Isolation":** Induces high voltage in Branch #1 and verifies Branch #2 remains entirely isolated and unaffected.
- **11. Test "Trunk vs Tuft Penalty":** Compares ATP drop between `Apical Trunk` (expensive active transport) and `Apical Tuft` (cheap). Trunk must burn more ATP.
- **12. Test "Branch Refractory Period":** Triggers an NMDA spike and verifies the branch ignores further input for exactly `BRANCH_REFRACTORY_TICKS` (3).
- **13. Test "Passive Leak Diffusion":** Checks that a small amount of charge (`PASSIVE_LEAK_PERCENT`) diffuses into the soma every tick from active branches.

## Block 3: Dynamics of Learning (STDP & Dopamine)

*Goal: Ensure synapses act as living, adaptable memory.*

- **14. Test "Classic LTP":** Source spikes shortly before Target. Verifies synaptic weight increases via Long-Term Potentiation.
- **15. Test "Classic LTD":** Source spikes long after Target. Verifies synaptic weight decreases via Long-Term Depression.
- **16. Test "Dopamine Amplification":** Sets `dopamine_multiplier` to 10.0. Verifies LTP weight growth is massively accelerated.
- **17. Test "Synaptic Scaling (High Activity)":** Forces target to fire 50+ times per second. Verifies homeostatic `SYNAPSE_PENALTY` begins cutting all incoming weights.
- **18. Test "Reliability Fade":** Uses a critically weak synapse (`weight = 0.01`). Verifies `failure_roll` prevents neurotransmitter delivery (unreliable transmission).
- **19. Test "Receptor Saturation":** Floods the cleft with 200 vesicles. Verifies the `effective_signal` is capped by the receptor count (synaptic weight).

## Block 4: Profiling & Hardware (The Squeeze)

*Goal: Find the limits of the CPU/GPU and system architecture.*

- **20. Test "Linear Scaling (Neurons)":** Measures TPS for 1k, 5k, 10k, 50k neurons with fixed 10 synapses each to verify Rayon/CUDA scalability.
- **21. Test "Connectivity Explosion":** 1000 neurons, scales synapses from 10k to 1M. Detects limits of memory bandwidth.
- **22. Test "FFI Overhead":** Isolates and measures the pure overhead of crossing the Rust <-> C++ boundary via FFI.
- **23. Test "JSON Saturation":** Decreases the UI snapshot interval from 100 down to 1. Locates the exact breaking point where `serde_json` chokes the engine.
- **24. Test "Memory Footprint":** Allocates 100,000 neurons and verifies RAM stability (Leak detection).
- **25. Test "Double Buffer Sync":** Confirms that spikes generated in `tick_neurons` are immediately visible to `tick_synapses` in the exact same engine cycle.

## Block 5: Specialized Cells & Behavior

*Goal: Validate the unique "character" of different cell types.*

- **26. Test "Pacemaker Rhythm":** Verifies `TYPE_PACEMAKER` cells automatically depolarize and fire rhythmically without external stimuli.
- **27. Test "Photoreceptor Dark Current":** Verifies `TYPE_PHOTORECEPTOR` depolarizes to `-40mV` in total darkness.
- **28. Test "Photoreceptor Light Bleach":** Verifies Photoreceptors hyperpolarize to `-65mV` in full light.
- **29. Test "Nociceptor Sensitization":** Applies continuous stress to a `TYPE_NOCICEPTOR` and verifies its activation threshold drops (becomes hypersensitive).
- **30. Test "Purkinje Output Integrity":** Verifies `TYPE_PURKINJE` to Motor connections maintain absolute silence when unstimulated (no random noise).

## Block 6: Global Cycles (Sleep & Evolution)

*Goal: Verify long-term survival and network structural plasticity.*

- **31. Test "Fatigue Sleep Trigger":** Artificially inflates metabolic wear across the network and ensures `is_sleep_needed()` returns true.
- **32. Test "Synapse Pruning":** Triggers the Microglia sleep cycle and ensures synapses with `weight < 0.01` are permanently deleted.
- **33. Test "Neurogenesis Rate":** Simulates 500+ neurogenesis cycles. Verifies new synapses form between active, physically close neurons without exceeding the hard limits per cell.
- **34. Test "Dopamine Decay":** Injects a massive dopamine rush (8.0) and verifies it decays back to baseline (1.0) linearly over exactly ~140 ticks.
- **35. Test "Pavlovian Success Rate":** Simulates a full training cycle (Sensor -> Relay -> Motor) under high dopamine. Verifies LTP locks the pathway in, allowing the Sensor to trigger the Motor autonomously after training.

## Block 7: Low-Level Benchmarking (O(n^2) Detection)

*Goal: Automated detection of non-linear scaling and bottlenecks.*

- **36. Test "The Pure Grid":** Scales empty neurons 1000 -> 2000 -> 4000 -> 8000. Asserts that the spatial search in `run_neurogenesis_service` scales linearly (< 3.5x factor per 2x size), proving our O(N) optimization holds.
- **37. Test "Synaptic Saturation":** Scales synapses on 500 neurons to 100k. Monitors for Cache Miss cliffs when RAM bandwidth maxes out.
- **38. Test "Dendritic Branching Stress":** Measures engine TPS with 1, 4, and 8 active dendritic branches per neuron. Verifies non-linear summation does not cause massive Warp Divergence.
- **39. Test "Serialization Backpressure":** Aggressive UI streaming test tracking TPS drops as JSON payloads scale.
- **40. Test "Idle Drift":** Establishes the absolute baseline TPS for 10,000 empty neurons, isolating the fixed engine orchestration cost.

### STDP Tests (`test_stdp.rs`)

- **41. Test "test_memory_layout_consistency":** Validates that CPU and GPU structs maintain identical memory alignment.
- **42. Test "test_stdp_trace_decay":** Validates correct decay of the `spike_trace` over time.
- **43. Test "test_stdp_ltp_causal_firing":** Tests Long-Term Potentiation (LTP) under causal firing conditions (Source -> Target).
- **44. Test "test_stdp_ltd_acausal_firing":** Tests Long-Term Depression (LTD) under acausal firing conditions (Target -> Source).
- **45. Test "test_stdp_dopamine_scaling":** Checks the scaling of the learning rate under high dopamine levels.
- **46. Test "test_stdp_weight_clamping":** Ensures synaptic weights do not exceed `MAX_SYNAPSE_WEIGHT` or drop below 0.01.
- **47. Test "test_stdp_associative_persistence_under_noise":** Hardcore stress test for Pavlovian associative persistence under heavy neural noise.
- **48. Benchmark "bench_stdp_overhead":** Benchmarks the overhead of calculating STDP during simulation.

### Network Generation Tests (`network_generation_tests.rs`)

- **49. Test "test_network_generation_proportions":** Validates strict neural type proportions (Pyramidal 75%, Interneurons 15%, Sensors 4%, Motors 4%, Pacemakers 2%) and the generation of inhibitory (negative) weights for interneurons.

### MMN v1.1 Validation Suite (neuron_physics_tests.rs)

- **50. Test "ATP-Gating (Starvation Filter)":** Ensures a starving neuron naturally increases its threshold and ignores weak stimuli.
- **51. Test "Phase Resonance (Internal Clock)":** Validates that signals propagating between neurons with matching internal phases are amplified, while opposite phases cause destructive interference (signal cancellation).
- **52. Test "Precise STDP (Targeted Dendritic Learning)":** Proves that learning (LTP) only happens on synapses connected to dendritic branches that actually contributed to a somatic spike.
- **53. Test "NMDA Multiplexing (Logical AND)":** Checks dendritic logic: distributed weak signals do not trigger a spike, whereas the exact same signals concentrated on one branch cross the `nmda_threshold` and force a somatic spike.
- **54. Test "The Matrix (Golden Master Snapshot)":** Generates and asserts a 100-tick lifecycle of a Pyramidal neuron to prevent any mathematical regressions in future engine updates.

### Environment Regulator Tests (`regulator_tests.rs`)

- **55. Test "test_regulator_perfect_conditions":** Feeds perfect tick latencies (1.0ms) and ensures Oxygen and Glucose remain at 1.0 (100%).
- **56. Test "test_regulator_sudden_lag":** Feeds a single massive latency spike (10.0ms) and verifies Oxygen drops to 0.0 instantly, while Glucose remains high due to EMA smoothing.
- **57. Test "test_regulator_chronic_lag":** Simulates 500 ticks of chronic lag and verifies both Oxygen and Glucose hit rock bottom.
- **58. Test "test_homeostasis_loop_integration":** Integrates the regulator into a live simulation loop, mocks hardware lag, and verifies the environment state dynamically updates based on actual execution time.

## Block 8: Network Vitality & Diagnostics

*Goal: Ensure the network stays dynamic, biological, and performs optimally.*

- **59. Test "True Snapshot Integrity":** Creates a specific network topology and specific `Environment` levels (e.g. Oxygen 0.42). Serializes via `BrainSnapshot` and deserializes. Asserts that exact `current_tick` and `Environment` levels are perfectly preserved to prevent "Amnesia".
- **60. Test "Performance Scaling TPS":** Runs network loads of varying sizes (up to 100k neurons / 1M synapses) for a small tick count. Verifies the Ticks Per Second (TPS) metric dynamically, preventing accidental inclusion of extremely slow logic in hot paths.
- **61. Test "Circadian Startup Grace":** Simulates extreme hardware latency during the first 100 ticks. Asserts that the Environment Regulator maintains 1.0 oxygen (Grace Period). Fast-forwards to tick 600 with the same latency and asserts that hypoxia correctly triggers.
- **62. Test "Thermal Noise Prevents Freezing":** Isolates a single Pyramidal neuron. Runs ticks without stimuli. Asserts `current_voltage` does not stagnate perfectly at `resting_voltage` but fluctuates due to biological thermal noise.
- **63. Test "Phoenix Resurrection":** Artificially kills a neuron. Verifies it remains completely inactive for exactly 999 ticks. On tick 1000, asserts that the neuron revives, restores its ATP, and resumes `is_alive = true`.
- **64. Test "HTM Predictive State Coincidence":** Validates Jeff Hawkins' active dendrites theory. Proves that contextual signals on Basal dendrites set the neuron into a "Predictive State" for 20 ticks. Verifies that a weak Proximal signal correctly triggers an action potential only while the predictive window is open (Coincidence Detection).

### 65. `test_atomic_hammer`

**Location:** `tests/edge_cases_tests.rs`
**Purpose:** Ensure `atomic_add_packed_voltage` reliably handles 512 concurrent sources spiking onto a single dendrite without data loss.

### 66. `test_sequence_recall`

**Location:** `tests/edge_cases_tests.rs`
**Purpose:** Verify HTM active dendrites. A -> B -> C forms a predictive chain. Spiking A causes B to enter predictive state, which then correctly propagates sequence memory without misfiring.

### 67. `test_seizure`

**Location:** `tests/edge_cases_tests.rs`
**Purpose:** Validate Homeostatic E/I balance. Provoking a massive spike wave on 50% of a 1000-neuron network should automatically regulate and calm down back to background noise levels within 500 ticks.

### 68. `test_signal_race_condition`

**Location:** `tests/edge_cases_tests.rs`
**Purpose:** Verify accurate synapse delivery delays. An unmyelinated spike (4 ticks) and myelinated spike (1 tick) triggered with precise offset must arrive and sum perfectly on the identical tick.

### 69. `test_metabolic_darwinism`

**Location:** `tests/edge_cases_tests.rs`
**Purpose:** Check long-term structural stability. Over 10,000 ticks of neurogenesis and microglial pruning, the total synapse count must remain stable and not cascade to 0 or infinity.

### 70. `test_inhibitory_stdp`

**Location:** `tests/edge_cases_tests.rs`
**Purpose:** Validate inhibitory plasticity. If a target is hyperactive, the inhibitory synapses connected to it must strengthen to suppress the activity.

### 71. `test_no_orphan_neurons`

**Location:** `tests/columnar_tests.rs`
**Purpose:** Ensure that network generation eliminates orphan neurons. All neurons must have at least 3 outgoing synapses, and neurons in Layer 4 must project forward to Layer 2/3.

### 72. `test_wta_suppression`

**Location:** `tests/columnar_tests.rs`
**Purpose:** Validate Winner-Takes-All (WTA) lateral inhibition in minicolumns. Ensures that when one neuron in a minicolumn spikes, other neurons in the same minicolumn are suppressed (hyperpolarized and flagged as inhibited).

### 73. `test_sdr_activity_limit`

**Location:** `tests/sparsity_tests.rs`
**Purpose:** Verify Sparse Distributed Representation (SDR) tuning mechanism. Proves that if Layer 4 activity exceeds the target sparsity of 5%, homeostatic scaling dynamically raises the activation threshold to enforce sparsity.

### 74. `test_sequence_learning`

**Location:** `tests/htm_tests.rs`
**Purpose:** Verify HTM sequential memory. Repeatedly firing Pattern A followed by Pattern B causes Pattern B's neurons to enter a Predictive State upon seeing Pattern A alone.

### 75. `test_column_bursting`

**Location:** `tests/htm_tests.rs`
**Purpose:** Validate Minicolumn Bursting. When a new, unpredicted stimulus arrives at a minicolumn and no neuron is in a Predictive State, the entire minicolumn fires simultaneously (Bursting).

### Logic Integrity Tests (`logic_integrity.rs`)

- **76. test_prediction_decay:** Validates that HTM predictive state correctly decays after 20 ticks.
- **77. test_sequence_learning_and_surprise:** Ensures sequence learning works, and unexpected stimuli trigger Minicolumn Bursting.
- **78. test_cross_column_voting:** Verifies that multiple neurons in different columns voting for the same predictive state mutually reinforce each other.
- **79. test_metabolic_fatigue:** Validates that high continuous firing reduces ATP, eventually triggering fatigue and dynamic threshold increases.

### Classifier & Functional Testing (`test_stdp.rs` / `modes.rs`)

- **80. test_mnist_classifier_competition:** Validates Winner-Take-All (WTA) logic. Simulates output neurons competing; ensures the neuron with the highest voltage/trace wins, and all others are penalized heavily (-20mV) and flagged with `FLAG_IS_INHIBITED`.
- **81. test_stdp_ltd_forgetting:** Validates LTD-based forgetting mechanism. Proves that if a target neuron spikes repeatedly while a specific source neuron remains silent, the synapse weight from that source correctly decreases over time.

### Block 9: Core Biological & Functional Mechanics (Phase 16)

#### Connectivity & Reaction (`func_connectivity_tests.rs`)
- **82. test_live_wire_propagation ("Живой провод"):** Проверяет базовую проходимость сигнала от L4 до L5 слоев.
- **83. test_autonomous_predictor ("Автономная предсказалка"):** Валидирует переход нейрона в предиктивное состояние при получении сигнала на базальные дендриты.
- **84. test_biological_wta ("Биологическое WTA"):** Тестирует механизм латерального торможения (победитель получает все, остальные пенализируются).
- **85. test_dopamine_instinct ("Дофаминовый инстинкт"):** Убеждается, что правильное угадывание (каузальный спайк) приводит к скачку дофамина (положительное подкрепление).
- **86. test_instant_karma ("Мгновенная карма"):** Убеждается, что ошибочный ответ резко снижает уровень дофамина (отрицательное подкрепление).

#### Topology & Discipline (`func_topology_tests.rs`)
- **87. test_dopamine_washout ("Чистая память"):** Гарантирует, что уровень дофамина со временем плавно возвращается к базовому (1.0).
- **88. test_branch_diversity ("Равномерный посев"):** Проверяет, что синапсы равномерно распределяются по дендритным ветвям без перекосов.
- **89. test_geometric_justice ("Геометрическая справедливость"):** Валидирует закон $L^{-1.5}$: вероятность создания синапсов падает с расстоянием, коротких связей экспоненциально больше.
- **90. test_synaptic_scaling_proportionality ("Автоматический гомеостаз"):** Гарантирует пропорциональное масштабирование весов при избыточной активности для поддержания баланса.
- **91. test_echo_decay ("Затухание эха"):** Проверяет E/I баланс — волна активности должна быстро затухать благодаря интернейронам, не приводя к эпилепсии.

#### Biology & Logic (`func_biology_tests.rs`)
- **92. test_photoreceptor_transduction ("Светобоязнь"):** Валидирует плавную гиперполяризацию фоторецепторов при попадании "света" (внешнего стимула).
- **93. test_nmda_multiplexing ("Эффект совпадения"):** Проверяет логическое И (AND) — нейроны в предиктивном состоянии срабатывают легче за счет дополнительного буста напряжения.
- **94. test_metabolic_death ("Ресурсный голод"):** Гарантирует падение нейрона в кому или смерть при исчерпании АТФ и отсутствии кислорода/глюкозы.
- **95. test_synaptic_armor ("Синаптическая броня"):** Проверяет, что сильные (консолидированные) связи теряют вес при LTD значительно медленнее, чем слабые.
- **96. test_pathfinding_audit ("Проверка проводов"):** Аудит графа — подтверждает, что сеть от сенсоров до выходного слоя не имеет "мертвых зон" и способна проводить сигналы.
