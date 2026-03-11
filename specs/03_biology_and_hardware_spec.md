# Biological & Hardware Specifications

## 1. Hardware (Physical Components)

* **Dendrites:** Input antennas. Catch chemistry from neighbors. In software: Input layers. Receive signals multiplied by weights.
* **Soma (Body):** Main workshop. Supports life, accumulates local potentials. In software: Base node. Stores current charge (voltage).
* **Nucleus:** Stores DNA. Issues blueprints for new proteins/receptors. In software: Long-term configuration database.
* **ER + Ribosomes:** Assembly line. Builds proteins according to blueprints from the nucleus. In software: Instance factory (creating new receptor objects).
* **Golgi Apparatus:** Sorts and packages proteins/chemistry into vesicles. In software: Preparation of data packets for sending.
* **Axon Hillock:** Calculator. Sums pros and cons. If threshold is breached — fires. In software: Activation function with a hard Threshold.
* **Axon:** Main cable. Conducts electrical spike. In software: Signal transmission bus (without power loss).
* **Telodendria:** Termination of the cable into thousands of small wires at the end. In software: Output divergence multiplier. One signal flies to many.
* **Synaptic Terminal:** Sprayer. Turns current into chemistry release. In software: Data sending interface (Output port).

## 2. Power and Support (Cell DevOps)

* **Mitochondria (ATP):** Make energy from oxygen and sugar. Consequences of failure: No ATP -> pumps stop -> node death.
* **Sodium-Potassium Pump:** Consumes 50-70% of energy, maintains baseline charge (-70 mV). Consequences of failure: Loss of potential difference (short circuit).
* **Excitotoxicity:** Glutamate overdose -> high calcium influx -> cell digests itself. Consequences: Apoptosis (quiet suicide) triggers.
* **Autophagy:** Lysosomes eat and recycle cellular waste. Consequences of failure: Accumulation of bugs and toxic proteins.
* **BDNF:** "Fertilizer" for good work. Blocks suicide, helps grow new branches.
