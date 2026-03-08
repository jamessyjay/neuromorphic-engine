# Heterogeneity, Evolution, and Validation Protocol

This document describes the transition from "ideal clones" to biological diversity and systems verification methods.

## 1. Heterogeneity (Initial Randomization)
In nature, no two neurons are the same. When initializing a neuron via `init_neuron`, parameters should NOT be constants. A Gaussian Distribution is used.

**Formula:** $$Value_{final} = \mu + \sigma \cdot N(0, 1)$$

| Parameter | Base ($\mu$) | Deviation ($\sigma$) | Meaning |
| :--- | :--- | :--- | :--- |
| `resting_voltage` | -70.0 mV | 2.5 mV | Different "resting" levels |
| `activation_threshold`| -55.0 mV | 1.5 mV | Different "excitability" |
| `energy_atp_max` | 100.0 | 5.0 | Different metabolic tank capacities |
| `leak_efficiency` | 1.0 (coeff) | 0.1 | Pump efficiency |

## 2. Evolution and Adaptation (Homeostatic Plasticity)
A neuron must change its parameters "on the fly" (within the CUDA kernel) to survive and operate effectively.

- **Threshold Scaling:** If the average spike frequency over the last 1000 ticks exceeds a limit — the neuron smoothly raises its `activation_threshold` (becomes "thick-skinned").
- **Metabolic Stress:** If ATP frequently drops below 20%, the efficiency of ATP generation by mitochondria falls (wear-and-tear effect).
- **BDNF Growth:** Upon successful learning (STDP), neighboring Astrocytes release growth factor, which increases the `receptor_count` (weight) of synapses.

## 3. Extended Use Cases (Stress-Tests)

### Use Case 1: Synaptic Fatigue
- **Stimulus:** Applying a 100 Hz signal to a single dendrite.
- **Expectation:** Depletion of the `vesicles_pool`. The effective signal at the output should fade, even if the input is still "hot."

### Use Case 2: Hypoxic Shutdown (Death from suffocation)
- **Stimulus:** `Environment.global_oxygen_level = 0.05` for 500 ticks.
- **Expectation:** ATP collapse -> pumps stop -> voltage drifts toward 0 -> `die_gracefully` call.

### Use Case 3: Epileptic Feedback (Looping)
- **Stimulus:** Connecting three Pyramidal neurons in a ring with a weight of 5.0.
- **Expectation:** Uncontrolled self-acceleration -> calcium explosion (`calcium_level >= 100`) -> death of the entire group.

## 4. Validation (Digital Twin)
To confirm model accuracy, simulation results (CSV/Charts) must be compared with real data (e.g., from the Allen Brain Institute).

**Success Metrics:**
1. **Spike Shape:** The spike curve must have phases of depolarization, repolarization, and aftercurrent hyperpolarization (dip to -80mV).
2. **Frequency-Current (F-I) Curve:** The dependence of firing frequency on the strength of the incoming current should be logarithmic, not linear.
3. **Heatmap Visualization:** On the network graph, neurons should be colored based on state (White: spike, Red: calcium stress, Gray: death).