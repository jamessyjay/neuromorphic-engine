# OOP Architecture

## 1. Base Class: `BaseNeuron`
**Properties:**
* `id`: Unique node identifier.
* `size`: Soma size (default is average).
* `resting_voltage`: Baseline charge (default -70.0 mV).
* `activation_threshold`: Firing threshold (default -55.0 mV).
* `spike_peak`: Spike peak (default +40.0 mV).
* `energy_ATP`: Current energy level (from 0 to 100%).
* `is_alive`: Boolean value (true/false).
* `inputs_array[]`: Array of incoming connections (dendrites).
* `outputs_array[]`: Array of outgoing connections (axon -> telodendria).

**Methods:**
* `calculate_sum()`: Add up all incoming pros and cons.
* `fire_spike()`: If sum > activation_threshold, fire current up to spike_peak.
* `release_neurotransmitter(type, amount)`: Release chemistry into the synapse after a spike.
* `apply_cooldown()`: Block fire_spike() for 1-3 ms (refractory period).
* `die_gracefully()`: Trigger apoptosis if energy_ATP == 0 or excessive calcium overload occurs.

## 2. Child Classes (Inheritors)
* **`PyramidalNeuron`:** Large (Pyramid). Base: -70 mV. Trigger: -55 mV. Main processor. Inherits everything from base. Has an exceptionally complex calculate_sum(), as there are thousands of inputs. Releases Glutamate (excites others).
* **`PurkinjeCell`:** Huge (2D fan). Base: -70 mV. Trigger: -50 mV. Parallel calculator. inputs_array increased to 200,000. Overridden firing method: can do a regular fire_spike(), or fire_complex_spike() (powerful weight reset for timing training).
* **`BipolarNeuron`:** Small (Wire). Base: -70 mV. Trigger: -55 mV. Relay node. Strictly 1 input and 1 output. calculate_sum() is almost unnecessary. Simply receives packet -> passes packet. Maximum speed.
* **`Interneuron`:** Small (Bush). Base: -70 mV. Trigger: -55 mV. Local brake. Lives only locally. Always calls release_neurotransmitter(GABA), distributing negatives to all neighbors to cut out noise and dampen epilepsy.
* **`PacemakerNeuron`:** Medium. Base: Floating. Trigger: -40 mV. Clock frequency generator. calculate_sum() is ignored. Has a built-in while(is_alive) loop that constantly raises charge to -40 and fires. Sets the rhythm for the entire network.
* **`Photoreceptor`:** Small (Cone). Base: -40 mV. Trigger: NULL. Analog sensor. fire_spike() is DISABLED. instead, smooth_dimmer() is active. While dark — releases neurotransmitter continuously. As soon as light falls — charge drops to -65 mV, and chemistry release stops (inverted logic).
* **`Nociceptor`:** Medium (Long). Base: -65 mV. Trigger: Dynamic. Damage (pain) sensor. activation_threshold is not static. With frequent use, the threshold lowers (e.g., to -60 mV), and the sensor starts spamming alerts at the slightest touch (inflammation effect).

## 3. Service Classes (Daemons)
* **`Astrocyte`:** Garbage Collector (GC) & Resource Manager. Monitors synapses. If it sees excess Glutamate — calls clean_synapse(). Also boosts ATP (energy_ATP += 10) for active neurons.
* **`SchwannCell / Oligodendrocyte`:** Network Optimizer. Wraps outputs_array (axon) in myelin (insulation). Changes transmission physics: instead of slow channel-by-channel calculation, enables teleport_signal(), speeding up data packets 100x.
* **`Microglia`:** Antivirus / Firewall. Swims through the network. Finds dead objects (is_alive == false), calls delete(), and clears memory of trash.
## 4. Memory Geometry & Serialization (Phase 34 & 35)
* **`BaseNeuron` Alignment:** Strictly packed and padded to exactly `128 bytes` (2 cache lines) with `alignas(64)` in C++ and `#[repr(align(64))]` in Rust to eliminate cache latency bottlenecks.
* **Fixed-Point Arithmetic:** `Compartment` values (like `voltage` and `nmda_threshold`) are packed into 16-bit integers (`uint16_t`) mapped to a 0.0-100.0 float range. This compresses tree size from 96 bytes down to 64 bytes.
* **Serialization (`serde`/`bincode`)**: The entire state of the brain is serialized and deserialized to disk (`brain_save.bin`). The strict C ABI representation ensures pointerless recreation of the exact CUDA memory map instantly upon engine restart.
* **Cryptographic Observability:** A SHA256 deterministic fingerprint is generated from neural states and synaptic topologies to mathematically prove structural integrity across the memory-disk serialization boundary.
