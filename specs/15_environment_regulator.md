# **Spec: Environment Regulator (Cyber-Biological Homeostasis v1.0)**

## **1. Concept and Role (Why we need this)**

The **Environment Regulator** is a global system controller within the Rust wrapper that acts as the "heart and lungs" of the simulation.

Its primary goal is to protect the physical hardware (CPU/GPU) from overloads, freezes, and Thermal Throttling by translating hardware stress into biological language (hypoxia and starvation).

Instead of artificially capping FPS or throwing errors upon memory overload, we force the neural network itself to "suffocate" and reduce its activity (via the built-in ATP-Gating) when the hardware struggles. This forms the **perfect Negative Feedback Loop**.

---

## **2. Mechanics (How it is implemented)**

We utilize two resources from our `Environment` structure for different types of response: **Oxygen (Fast)** and **Glucose (Slow)**.

The Regulator measures the execution time of each simulation tick (Tick Latency, $\Delta T$) and compares it against a target value (e.g., 1 millisecond per tick).

### **2.1. Oxygen — Protection against spike avalanches (Fast-Twitch)**

Oxygen reacts **instantly** to the current system lag.

* **Trigger:** If the current tick takes longer than normal to compute (e.g., an avalanche of spikes clogs the memory bus).
* **Action:** The $O_2$ level drops directly proportional to the delay (down to 0.0).
* **CUDA Consequence:** As soon as $O_2 < 0.5$, the `hypoxia_penalty` kicks in. ATP burns rapidly across all neurons. Starving neurons raise their `activation_threshold` (our MMN v1.1 fix) and fall silent. The spike avalanche stops, and the hardware is freed up.

### **2.2. Glucose — Protection against chronic overheating (Slow-Twitch)**

Glucose reacts to the **Moving Average** of the tick time over the last 100-500 steps.

* **Trigger:** If the hardware consistently chugs near its limit (the network is too dense, too much background activity), even without sharp lag spikes.
* **Action:** The global Glucose level gradually decreases.
* **CUDA Consequence:** Bonus ATP regeneration is disabled (the line `if (env.global_glucose_level > 0.1f... )`). The network as a whole transitions into "Low Power Mode". The baseline pulsation frequency of the network drops.

---

## **3. Mathematics in Rust (Regulator Algorithm)**

In the main simulation loop (e.g., in `modes.rs` before calling `launch_tick_neurons`), we inject a micro-evaluation module:

**Parameters:**

* `TARGET_TICK_MS = 2.0` (Ideal tick time).
* `CRITICAL_TICK_MS = 10.0` (Time at which death/total suffocation occurs).

**Logic (Pseudocode):**

1. Record `start_time`.
2. Perform network tick (CUDA/FFI).
3. Record `end_time`. Calculate `delta_ms`.
4. Update average delay `avg_delta_ms` (e.g., SMA over 100 ticks).
5. **Calculate Oxygen (for the next tick):**
   $$O_2 = 1.0 - \left( \frac{\max(0, \Delta ms - TARGET\_TICK\_MS)}{CRITICAL\_TICK\_MS - TARGET\_TICK\_MS} \right)$$
   *(Clamp the result between 0.0 and 1.0)*
6. **Calculate Glucose (for the next tick):**
   $$Glucose = 1.0 - \left( \frac{\max(0, avg\_\Delta ms - TARGET\_TICK\_MS)}{CRITICAL\_TICK\_MS - TARGET\_TICK\_MS} \right)$$

---

## **4. Why this is fucking brilliant (Trade-offs)**

* **Zero Overhead in CUDA:** The GPU itself doesn't give a shit. It receives the `Environment` struct exactly as before. Latency calculations happen on the CPU in Rust in nanoseconds.
* **True Dynamics:** This is not a hardcoded limit. If you run the network on a beefy server with an RTX 5090, `delta_ms` will always be low, oxygen will stay at 1.0, and the network will blast at maximum spike density. If you run it on a Mac on battery power — the network itself will "shrink" and quiet down to avoid killing the laptop.
* **Emergence:** This gives the network the behavior of a true living organism. It literally **feels** the boundaries of its "universe" (your hardware) and adapts to them.
