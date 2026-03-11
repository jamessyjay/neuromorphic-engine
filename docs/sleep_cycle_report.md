# Phase 35: The Sleep Cycle (Structural Consolidation & Binary Serialization)

## Overview

Implemented a biological "Sleep Cycle" where the neuromorphic engine pauses active execution, prunes weak connections, recovers metabolic parameters, and serializes the exact state of the network to disk for immediate loading upon subsequent runs. 

## Architectural Features

1. **Serialization Layer (Serde + Bincode):**
   - Derived `Serialize` and `Deserialize` on all fixed-memory FFI structs (`BaseNeuron`, `Compartment`, `Synapse`, `Environment`).
   - The memory geometry `[Compartment; 8]` and manual paddings are flawlessly serialized as direct raw bytes, ensuring compatibility between C++ execution and Rust I/O without any overhead.

2. **Fatigue Threshold Monitoring:**
   - The graph calculates `avg_wear` across the network. If the average `metabolic_wear` exceeds `40,000` (out of `65,535` max limits), the network signals the need for sleep.

3. **Sleep Cycle Sequence:**
   - **Pruning:** Any synapse with a weight `< 0.05` is garbage-collected. This mimics synaptic pruning during REM sleep.
   - **Metabolic Recovery:** All `metabolic_wear` metrics are reset to 0, `energy_atp` is fully replenished, and `calcium_level` is cleared.
   - **Binary Snapshot:** The network is written to `brain_save.bin` using high-speed binary encoding.

4. **Network Awakening:**
   - Upon engine initialization, `NetworkGraph::load_from_disk()` attempts to read `brain_save.bin`.
   - If found, it bypasses the heavy initialization process (calculating Gaussian distributions, resolving initial random seeds) and directly loads the matured network memory state.

## Validation Results (`--sleep-test`)

1. **Initial Run:**
   - Generated an infant network of 1,000 neurons and 10,000 random synapses.
   - Artificially accelerated metabolic wear to force the fatigue limit rapidly.
   - Output confirmed entering the Sleep Cycle, pruning weak synapses, and serializing.
   
2. **Subsequent Run:**
   - Execution explicitly output: `Awakening network from brain_save.bin...`
   - Bypassed the `Generating Infant Network...` phase.
   - Resumed testing from the saved structural state, demonstrating total success of the biological sleep save states.
