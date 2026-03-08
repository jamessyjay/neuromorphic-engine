# Testing Protocol & Tasks

## Base Tests (Virtual Oscilloscope)
1. **Pumps Test (Leak):** Excite the cell to -60mV. Verify that the charge smoothly drains to -70mV, consuming `energy_ATP`.
2. **Summation (Spatial/Temporal):** Ensure that small excitatory signals add up and are capable of breaching the `activation_threshold`.
3. **Inhibition:** Apply GABA. Ensure that `current_voltage` drops below -70mV, blocking subsequent fires.
4. **Cooldown:** Ensure that calling `fire_spike()` is impossible during the `apply_cooldown()` period.
5. **Specific Tests:**
    - Verify analog operation of `Photoreceptor` (absence of spikes).
    - Verify infinite loop of `PacemakerNeuron`.

## Task Pool (Development Tasks)
1. Implement the `BaseNeuron` abstract class with all properties and methods.
2. Implement the internal cycle (Tick System), binding ATP consumption to `resting_voltage`.
3. Create child classes, overriding their parameters according to the architecture.
4. Implement service workers (`Astrocyte`, `Microglia`) for background memory and resource management.
5. Cover each class with tests from the Protocol. No commits until tests are green.