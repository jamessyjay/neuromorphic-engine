# Neuromorphic Engine - Comprehensive Testing Suite

This document outlines the testing strategy for the Neuromorphic Engine. It is divided into 7 blocks, covering everything from basic biological accuracy to low-level hardware performance and non-linear scaling detection.

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
