# Network Protocols and System Limits

This document defines the hard boundaries of the system. A neuron cannot fire indefinitely; it has physical limits regarding energy, interface throughput, and cellular toxicity levels.

## 1. Communication Protocol (`Synapse` Class)

Neurons do NOT call each other's methods directly. Data teleportation is forbidden. An intermediary `Synapse` object always stands between the output of Neuron A and the input of Neuron B.
* **Properties:** `delay_ticks` (transmission delay, default = 1 tick), `chemical_buffer` (packet queue).
* **Work Logic:**
  1. The axon of Neuron A calls the `Synapse.push(Glutamate_amount)` method.
  2. The synapse places this packet into the buffer for the duration of `delay_ticks`.
  3. On the next tick, the synapse transfers the packet to the dendrite of Neuron B: calling `NeuronB.receive_signal(Glutamate_amount)`.
* **Why:** This hardware-mimics the time it takes for chemistry to swim across the micro-abyss between cells.

## 2. ATP Economics (Energy Math)
Energy does not appear out of thin air. Without a strict balance, the network would devolve into permanent spike spam.
* `MAX_ATP` = 100.0%.
* **Generation:** `+1.0%` per tick (if `Environment.global_glucose > 0`).
* **Base Expenditure (Pumps):** `-0.1%` per tick (spent on maintaining `LEAK_RATE` and holding -70 mV).
* **Spike Expenditure (Fire):** `-20.0%` per single fire!
* **Consequence:** If a neuron fires 5 times in a row without a break, its ATP will drop to zero. The pumps will stop, and firing will physically cease until the mitochondria cook up new energy. The node "gets tired."

## 3. Receptor Saturation (Throughput Limit)

You cannot simply multiply the incoming signal by the weight indefinitely. If there are few "locks" (receptors), the extra "keys" (chemistry) go to waste.
* **Formula:** `Effective_Signal = MIN(Incoming_Neurotransmitter, Receptor_Count)`
* **Example:** If a neighbor goes crazy and spits out 50 units of Glutamate, but our receiving dendrite only has `Receptor_Count = 10`, then only 10 units of signal will pass inside.
* The remaining 40 units will hang in the synaptic cleft (in the `Synapse` object's buffer) until destroyed by the scavenger (`Astrocyte` class).

## 4. Death Mathematics (Calcium Overload)

Calcium is the critical damage counter. If a cell is heavily spammed, it literally cooks from the inside.
* Base `calcium_level` = 0.
* **Accumulation (Damage):** With each Spike OR during powerful NMDA receptor opening, `calcium_level += 10`.
* **Cooling (Regeneration):** Every tick, the cell tries to remove calcium: `calcium_level -= 1` (down to a minimum of 0).
* **Death Trigger:** If `calcium_level >= 100` — the fatal `die_gracefully()` function is called. The cell is permanently removed from the simulation (Apoptosis).