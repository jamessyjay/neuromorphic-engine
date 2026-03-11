# Specification 12: Structural Plasticity (Dynamic Wiring)

## 1. Overview
Biological brains are not static graphs. They undergo constant structural changes through **Synaptogenesis** (creation of new synapses) and **Pruning** (removal of weak or redundant synapses). This allows the network to self-organize and adapt to specific tasks without manual wiring.

## 2. Spatial Topology
To support distance-based connectivity, every neuron must have a spatial location in a 3D coordinate system.

### BaseNeuron Extension:
```cpp
typedef struct {
    // ... existing fields ...
    float x, y, z; // Spatial coordinates (unitless)
} BaseNeuron;
```

## 3. Synaptogenesis (Growth)
Creating new connections happens when two neurons are functionally related but not yet connected.

### Triggering Conditions:
1. **Physical Proximity**: The Euclidean distance between Neuron A and Neuron B must be within the `Search Radius` ($R_{search}$).
2. **Activity Correlation**: Both neurons must have spiked recently.
3. **Metabolic Availability**: The source neuron must have sufficient ATP ($ATP > 80.0$) to "grow" a new dendrite/axon.
4. **Probability**: Growth is a stochastic process with a configurable rate ($P_{growth}$).

### Formula:
$P(connect) = P_{growth} \times f(dist) \times f(correlation)$
Where $f(dist) = e^{-(dist^2 / 2\sigma^2)}$ (Gaussian falloff).

## 4. Pruning (Cleanup)
Removing "impotent" connections allows the network to stay efficient and free up memory.

### Conditions for Removal:
1. **Weight Threshold**: If $Weight < 0.01$.
2. **Stability Period**: The weight must have stayed below the threshold for a certain amount of time (e.g., 5000 ticks) to avoid deleting transiently depressed connections.
3. **Death**: If either source or target neuron is dead (`is_alive == false`).

## 5. Implementation Strategy
- **Pruning**: Handled by the **Microglia Service** in Rust. It filters the synapse vector periodically.
- **Synaptogenesis**: Handled by a new **NeuroGenesis Service** in Rust. 
    - For large N, it uses **Random Sampling** (checking random pairs) instead of $O(N^2)$ exhaustive search to maintain performance.
    - Rate: Runs every 1000-5000 ticks depending on complexity.

## 7. The Sleep Cycle & Metabolic Wear (Phase 35)
As the network operates, continuous spiking and calcium flux generate "metabolic wear" that degrades network efficiency. To prevent collapse, the network must sleep.

### Mechanisms:
1. **Metabolic Wear Tracking**: Every spike increases a 16-bit packed `metabolic_wear` counter. At a critical threshold, the network triggers a sleep cycle.
2. **Offline Replenishment**: During sleep, normal stimulation stops. All ATP is restored to max, calcium is flushed to 0, and refractory periods are cleared.
3. **Consolidation**: The sleep cycle triggers an immediate pass of the **Microglia Service** to aggressively prune weak synapses (e.g. `Weight < 0.05`), simulating synaptic downscaling during slow-wave sleep.

### Observability & Integrity:
Before and after sleep, the network calculates a cryptographic **Fingerprint (SHA256)** of its topology to guarantee structural consistency across the serialization boundary (`serde`/`bincode`). This ensures memory alignment and biological pointers remain 100% intact after the cycle.
