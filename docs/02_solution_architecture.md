# Neuromorphic Engine: Solution Architecture (Biological Logic)

Detailed description of the mathematical models and biological algorithms implemented in the engine.

## 1. Neuron Model (Modified LIF & Metabolism)

Our neurons are not just summation units, but living entities with limited resources.

### Metabolism and ATP (Spec 02/11)
Each neuron has an **ATP** charge (energy).
- **Generation:** Depends on `global_oxygen_level` and `metabolic_wear` (mitochondrial wear).
- **Consumption:** Baseline leakage (`leak_coeff`) + massive consumption during a spike (`ATP_SPIKE_COST`).
- **Death:** If ATP < 1.0, `die_gracefully()` is called, and the neuron ceases to function (Apoptosis).

### Heterogeneity and Chaos (Spec 11)
In nature, no two neurons are the same.
- **Gaussian Heterogeneity:** Upon creation, parameters (`resting_voltage`, `threshold`, `max_atp`) are generated via a Gaussian distribution.
- **Stochastic Noise:** Every tick, stochastic noise (Gaussian) is mixed into the voltage, simulating the thermal noise of ion channels.

## 2. Synaptic Dynamics (Chemical Delivery)

### Neurotransmitter Buffer (Spec 06)
When a neuron spikes, the synapse does not instantly transmit the signal.
1. A spike releases a "portion" into the `neurotransmitter_buffer`.
2. Delay: The signal travels along the axon (0 ticks for myelin, 2 ticks without it).
3. Receptors: The signal is read from the buffer.

### Receptor Saturation (Spec 08)
A synapse has a conductance limit (number of receptors) determined by its `weight`.
- If there is more chemical in the buffer than the synapse can accept, the signal is limited (`receptor_limit`).
- Excess remains in the buffer until the next tick.

## 3. Plasticity (Learning)

### Dopamine-Modulated STDP (Phases 13-14)
Learning (weight change) occurs according to the STDP (Spike-Timing Dependent Plasticity) rule, but with a dopamine gate.
- Weight grows if the target spikes shortly after synapse activation.
- The `dopamine_multiplier` scales this growth. At low dopamine, learning is "frozen".

## 4. Homeostatic Plasticity

If a neuron spikes too frequently, its `activation_threshold` dynamically increases (desensitization). If the neuron is "silent", the threshold slowly decreases, making the neuron more sensitive (sensitization).

## 5. Wear & Aging

**Mitochondrial Wear:** With each spike, the `metabolic_wear` parameter increases.
- The higher the wear, the slower ATP regenerates.
- This limits the maximum discharge frequency of the neuron over long periods and simulates biological fatigue.
