# Mechanics and Math Logic (How it actually works)

This document describes the exact mathematical rules and logic of component operation. It is forbidden to use standard hacks from AI frameworks. All physics is written from scratch according to these rules.

## 1. Global Environment (Environment Singleton)
A neuron does not exist in a vacuum. The project must have a global `Environment` class (Bloodstream/Matrix).
* **Properties:** `global_glucose_level`, `global_oxygen_level`, `toxin_levels`.
* **Functionality:** In each tick, this singleton distributes resources to all active neurons.
* **Why:** If we want to test death from suffocation (Hypoxia), we simply set `Environment.global_oxygen_level = 0`, and all mitochondria in all neurons synchronously stop receiving fuel.

## 2. Charge Leak Mathematics (Leak Rate)
Pumps pull the charge toward the `RESTING_VOLTAGE` (-70.0 mV) **from both sides**.
* **Logic:** In each tick, if there are no external signals:
  * If `current_voltage` > -70.0 (excited neuron): `current_voltage -= LEAK_RATE`
  * If `current_voltage` < -70.0 (inhibited by GABA): `current_voltage += LEAK_RATE`
* This simulates the expenditure of ATP to equalize the balance. If `current_ATP == 0`, leakage stops, and the charge slowly drifts toward 0 mV (death).

## 3. Input Calculation (Transmitter Math)
Incoming signals are divided into terms and factors.
* **Base Transmitters (Terms):**
  * Glutamate: adds voltage (e.g., `+5.0 mV` * `receptor_count`).
  * GABA/Glycine: subtracts voltage (e.g., `-5.0 mV` * `receptor_count`).
* **Modulators (Multipliers/Coefficients):**
  * Dopamine: does not change the charge by itself! It acts as a multiplier.
  * *Formula:* `Total_Input = (Sum(Glutamate) - Sum(GABA)) * Dopamine_Multiplier`
  * If Dopamine is high (`Multiplier = 1.5`), the neuron reacts to Glutamate one and a half times more sharply.

## 4. Cooldown Logic (Absolute vs Relative)
A neuron has two charging phases after firing (`fire_spike`).
* **Absolute Cooldown (1 ms):** The `receive_signal()` method simply `returns`. The neuron is deaf and blind. The charge is strictly fixed at -80.0 mV.
* **Relative Cooldown (Next 3 ms):** The neuron begins to accept signals, BUT its `activation_threshold` is dynamically raised.
  * *Example:* Base threshold -55 mV. In the relative cooldown, it becomes -40 mV. To breach it, neighbors will have to pour in an enormous amount of Glutamate. As the timer progresses, the threshold smoothly returns to -55 mV.

## 5. Plasticity and Memory (`update_weights` method)
The neuron must be able to "remember" important connections in hardware (Hebb's Rule / STDP).
* **Trigger:** Coincidence of two events within a 10-20 millisecond window:
  1. Glutamate arrived at a specific dendrite (input).
  2. The neuron ITSELF issued a Spike (`current_voltage` broke the threshold).
* **Action (NMDA-cascade):** If both conditions are met, `update_weights(dendrite_id)` is called. The `receptor_count` variable (weight) for this specific input increases permanently by `0.01`. Next time, the signal from this neighbor will hit harder.