# Neuromorphic Engine: Solution Architecture (Biological Logic)

Detailed description of the mathematical models and biological algorithms implemented in the engine.

## 1. Neuron Model (Modified LIF & Metabolism)

Our neurons are not just summation units, but living entities with limited resources.

### Metabolism and ATP (Spec 02/11)

Each neuron has an **ATP** charge (energy).

- **Generation:** Depends on `global_oxygen_level` and `metabolic_wear` (mitochondrial wear). Accelerated during early "wakefulness".
- **Consumption:** Baseline leakage (`leak_coeff`) + massive consumption during a spike (`ATP_SPIKE_COST`).
- **Shutdown & Death:**
  - If ATP drops below critical levels (`< 15.0`), the neuron enters a metabolic shutdown (State 4), unable to spike until ATP recovers.
  - If ATP reaches `0.0`, or calcium toxicity gets too high, `die_gracefully()` is called.

### The Phoenix Loop (Resurrection)

To prevent permanent network degradation, death is not always final.

- Dead neurons enter a coma state (`dead_ticks`).
- After 1000 ticks of rest, the neuron "resurrects" with a fully restored ATP pool, allowing the brain to heal dynamically over long simulations.

### Heterogeneity and Chaos (Spec 11)

In nature, no two neurons are the same.

- **Gaussian Heterogeneity:** Upon creation, parameters (`resting_voltage`, `threshold`, `max_atp`) are generated via a Gaussian distribution.
- **Thermal Noise:** Every tick, stochastic noise (PCG32-lite pseudo-randomness) is mixed into the voltage, simulating the thermal noise of ion channels. This prevents the network from freezing into permanent silence.

## 2. Active Dendrites & HTM Predictive States

The engine implements advanced dendritic logic inspired by the Thousand Brains Theory (HTM). Summation is non-linear across 8 independent dendritic branches.

- **Proximal Dendrites (Branch 0):** Act as direct feedforward inputs. Exceeding the NMDA threshold here causes massive somatic depolarization (triggering a spike).
- **Basal/Apical Dendrites (Branches 1-7):** Act as contextual predictors. Exceeding the NMDA threshold here does *not* depolarize the soma directly. Instead, it places the neuron into a **Predictive State** for 20 ticks.
- **Coincidence Detection:** If a feedforward signal hits Branch 0 while the neuron is in a Predictive State, the required voltage threshold to trigger an action potential is significantly reduced (`+15.0 mV` virtual offset).

## 3. Synaptic Dynamics (Chemical Delivery)

### Neurotransmitter Buffer (Spec 06)

When a neuron spikes, the synapse does not instantly transmit the signal.

1. A spike releases a "portion" into the `neurotransmitter_buffer`.
2. Delay: The signal travels along the axon (0 ticks for myelin, 2 ticks without it).
3. Receptors: The signal is read from the buffer.

### Receptor Saturation (Spec 08)

A synapse has a conductance limit (number of receptors) determined by its `weight`.

- If there is more chemical in the buffer than the synapse can accept, the signal is limited (`receptor_limit`).
- Excess remains in the buffer until the next tick.

## 4. Plasticity & Lifelong Learning

### Dopamine-Modulated STDP

Learning (weight change) occurs according to strict Spike-Timing Dependent Plasticity (STDP), gated by causality and dopamine.

- **LTP (Potentiation):** Weight grows if the source spiked strictly *before* the target spiked. Simultaneous spikes do not cause instant potentiation to prevent paradoxical feedback loops.
- **Dopamine Scaling:** The `dopamine_multiplier` scales this growth. At low dopamine, learning is "frozen". Reward stimuli (Pavlovian conditioning) trigger dopamine rushes.

### Neurogenesis (Structural Growth)

The network topology is not static. Highly active neurons in close spatial proximity (`CLUSTERING_RADIUS_SQ`) and with sufficient ATP reserves have a stochastic probability to grow new synaptic connections during the `Neurogenesis Service` tick.

### Microglia (Probabilistic Pruning)

A background immune service that prunes weak or inactive synapses.

- **Probabilistic Drop:** Soft pruning based on weight and ticks since the last activation. Weak synapses have a chance to survive instead of being instantly deleted.
- **Competitive Retention:** A neuron is guaranteed to maintain at least `MIN_SYNAPSES_PER_NEURON` (e.g., 3), ensuring the network never fragments into completely isolated islands.

### Nursery Protection Phase

During the first 5000 ticks of a network's life, it undergoes a "Nursery" phase. `STDP_LEARNING_RATE` is doubled, and neurons are protected from complete starvation. This allows random initial weights to consolidate into functional cores before aggressive pruning begins.

## 5. Global Cycles

### Homeostatic Plasticity

If a neuron spikes too frequently, its `activation_threshold` dynamically increases (desensitization) to prevent seizures. If the neuron is "silent", the threshold slowly decreases, making the neuron more sensitive (sensitization).

### Wear & Aging

**Mitochondrial Wear:** With each spike, the `metabolic_wear` parameter increases.

- The higher the wear, the slower ATP regenerates.
- This limits the maximum discharge frequency of the neuron over long periods and simulates biological fatigue. Sleep cycles periodically flush out this wear.

## 6. Logic Integrity & HTM Integration

To ensure stable learning and continuous operation without mathematically crashing or saturating, the engine enforces strict logic constraints:

- **Sequence Learning & Surprise:** Predictable sequences cause specific neurons to anticipate signals via their distal dendrites. If a completely unpredicted signal (Surprise) hits a Minicolumn where zero neurons were in a predictive state, a **Minicolumn Burst** occurs (all neurons in the column fire) to represent the novelty of the stimulus.
- **Cross-Column Voting:** Sparse Distributed Representations (SDR) across the network build consensus. If multiple neurons across different minicolumns send contextual signals to a specific target column, those votes accumulate, reinforcing the correct prediction and suppressing noise.
- **Metabolic Fatigue:** Continuous bursting or high-frequency firing drains ATP faster than it can be restored. This creates a natural rate-limit, elevating the neuron's dynamic activation threshold and preventing runaway feedback loops (Epilepsy).

