# Neuron Specification (Full Biological Architecture)

## 1. Internal Factory (Organelles and States)
Each node has internal modules operating in the background:
- **Mitochondria:** Generate `ATP_energy` every unit of time (tick). Consume `glucose` and `oxygen` (global environment variables).
- **Sodium-Potassium Pumps:** Operate constantly. Spend `ATP_energy` to maintain `current_voltage` at the `RESTING_VOLTAGE` level.
- **Nucleus and ER (Endoplasmic Reticulum):** Database and factory. Create new receptors (changing synapse weights) if a signal arrives from metabotropic receptors.
- **Golgi Apparatus:** Packages neurotransmitters into vesicles (bubbles) for shipping.

## 2. Physical Constants
- `RESTING_VOLTAGE`: -70.0 mV.
- `THRESHOLD_VOLTAGE`: -55.0 mV.
- `SPIKE_VOLTAGE`: +40.0 mV.
- `REFRACTORY_PERIOD_ABSOLUTE`: 1 ms (Full block).
- `REFRACTORY_PERIOD_RELATIVE`: 3 ms (Reduced sensitivity, charge drops to -80.0 mV).

## 3. Dynamic Variables (State)
- `current_voltage`: Current charge.
- `current_ATP`: Energy level (0-100%). If it drops to 0 — pumps stop, `current_voltage` drifts towards 0, and the neuron stalls.
- `calcium_level`: Levels of calcium ions inside. If it exceeds a critical mark — a trigger for death (Apoptosis).
- `vesicles_pool`: Supply of neurotransmitter packets at the output.

## 4. Interaction Interfaces (Complex Ports)
- **Dendrites (Inputs):** Accept chemical signal objects (Glutamate, GABA, Dopamine). Each input has a `receptor_count` parameter (connection weight).
- **Axon Terminals (Outputs):** Upon Spike generation, they consume calcium and convert electricity into a chemical packet that is sent further.

## 5. Main Loop (Tick System)
In each tick (1 ms), the following occur in parallel:
1. **Metabolism Phase:** Mitochondria provide ATP, pumps consume ATP and balance the charge (Leak).
2. **Reception Phase:** Reading chemistry at the dendrites -> opening ion channels -> changing `current_voltage`.
3. **Calculation Phase (Axon Hillock):** Checking `current_voltage` against `THRESHOLD_VOLTAGE`.
4. **Reaction Phase:** If Spike -> voltage to +40, ATP deduction, cooldown initiation, chemistry release from terminals.