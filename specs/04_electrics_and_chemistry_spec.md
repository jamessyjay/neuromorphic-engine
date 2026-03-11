# Electrics, Timings, and Chemistry

## 1. Electrics and Timings
* **Resting Potential:** -70 mV. Baseline charge. Maintained by pumps (consumes significant energy).
* **Activation Threshold:** -55 mV. Trigger for firing. The sum of inputs must bring the charge to this level.
* **Spike (Fire):** +40 mV. Maximum current jump. Encodes the signal on an "All or Nothing" principle.
* **Spike Duration:** 1–2 ms. The time during which current physically travels along the cable within a single neuron.
* **Absolute Refractory Period:** ~1 ms. Channels are blocked. The neuron CANNOT fire under any circumstances.
* **Relative Refractory Period:** 2–4 ms. Charge dips (down to -80 mV). Firing is possible, but requires an exceptionally strong input.
* **Synaptic Delay:** 0.5–1 ms. The "Customs" check. Time for re-converting current into chemistry, traveling across the gap, and opening the neighbor's locks.

## 2. Network Interfaces and Memory (Synapse)
* **Vesicles + Calcium:** Signal transmission trigger. Current opens calcium channels -> calcium causes bubbles to release neurotransmitter.
* **Synaptic Cleft:** Transmission medium (physical gap). Micro-abyss between the terminal and the dendrite.
* **Ionotropic Receptors:** Fast if/else for data transfer. Fast locks. Chemistry opens the channel, ions change the charge.
* **Metabotropic (G-proteins):** Modification of network meta-parameters. Slow locks. Change internal cell settings, influence the nucleus.
* **NMDA Receptors:** Logical "AND". Basis for Hebb's rule. Require both external Glutamate and high internal charge to displace magnesium.
* **Plasticity (LTP/LTD):** Physical memory (Weights). Growing new receptors (strengthening) or hiding them (weakening).
* **Cleft Clearing:** Port reset to prevent signal stickiness. Reuptake (vacuuming) or destruction by enzymes.

## 3. Chemistry (Data Types)
* **Glutamate:** Main worker (Gas). + (Excitation). Breaches the threshold.
* **GABA / Glycine:** Main brakes. - (Inhibition). Dampen activity, push charge into negative territory.
* **Dopamine:** Modulator of focus and reward. Adjusts the "volume" of important signals.
* **Serotonin:** Background modulator. Regulates the overall state of the network (mood/sleep).
* **Acetylcholine:** Memory manager. Switches the network into "recording/learning" mode.
* **Norepinephrine/Epinephrine:** Alarm clock / Turbo button. Sharply narrows focus, cuts out excess noise.
* **Histamine:** System tone. Prevents the network from "falling asleep".
* **Endorphins (Peptides):** Built-in morphine. Cull overloads (pain), work slowly but massively.
## 4. Passive Leak & Refractory Physics (Phase 34)
* **Passive Leak:** Analog sub-threshold signals (below NMDA activation limits) are no longer discarded. A fraction of dendritic charge constantly "leaks" into the soma every tick. This solves the "digital deafness" problem, allowing weak but consistent signals to eventually build up a somatic potential.
* **Branch Refractory Period:** Once an NMDA spike triggers on a specific dendrite branch, that branch enters a localized refractory state (e.g. 3 ticks). Incoming neurotransmitters are consumed without generating further voltage, preventing "eternal high" infinite-spiking loops within a single tick window.
