# 🧠 Spec: Monolith Microchip Neuron (MMN v1.1 - Surgical Edition)

This specification describes the integration of the "Microchip Neuron" logic (metabolic gating, phase resonance, dendritic logic) into the existing biologically-plausible architecture without changing the memory layout sizes (Zero-Byte Growth).

## 1. Memory Layout

The total size of `BaseNeuron` remains strictly tied to **128 bytes** (2 GPU cache lines). We utilize existing padding bytes to store the new parameters.

### 1.1. Branch Structure (Compartment)

Size: 8 bytes (unchanged).
Changes: Replaced 1 byte of padding with `local_trace`.

| Parameter           | Type     | Size   | Description                                        |
|:--------------------|:---------|:-------|:---------------------------------------------------|
| `branch_type`       | `u8`     | 1b     | 0=Empty, 1=Basal, 2=Trunk, 3=Tuft                  |
| `refractory_timer`  | `u8`     | 1b     | "Rest" time of the branch after a local spike.     |
| `voltage`           | `u16`    | 2b     | Fixed-point branch potential.                      |
| `nmda_threshold`    | `u16`    | 2b     | Fixed-point threshold for NMDA spike.              |
| `local_trace`       | `u8`     | 1b     | **Precise STDP:** Trace of recent branch activity. |
| `_pad`              | `u8`     | 1b     | Alignment padding.                                 |

### 1.2. Main Structure (BaseNeuron - selected changes)

Size: 128 bytes (unchanged).
Changes: `_pad1` replaced with `internal_phase`.

| Parameter           | Type     | Size   | Description                                        |
|:--------------------|:---------|:-------|:---------------------------------------------------|
| ...                 | ...      | ...    | Existing fields (id, type_, is_alive, etc.)        |
| `internal_phase`    | `u8`     | 1b     | **Internal Clock:** Internal phase (0-255).        |
| ...                 | ...      | ...    | Existing fields (voltage, ATP, x,y,z, etc.)        |
| `branches`          | `Comp[8]`| 64b    | 8 dendritic branches (multiplexer).                |

---

## 2. Core Mechanisms (Implementation)

### 2.1. Multiplexing (Dendritic Logic Gates)

The built-in `branches` architecture is utilized as 8 independent input ports.
The AND logic gate is implemented via NMDA spikes:

* Weak signals on a branch only result in passive leak to the soma.
* Upon exceeding `nmda_threshold`, the branch generates a powerful local spike (`soma_impact = v * NMDA_SPIKE_GAIN`), which is almost guaranteed to break the soma's threshold.

### 2.2. ATP-Gating (Metabolic Threshold Control)

Dynamically increasing the soma's threshold during energy deficit.

**Formula in `process_spike`:**
$$Ratio_{energy} = \max(0.01, \frac{ATP}{ATP_{max}})$$
$$V_{threshold\_dynamic} = V_{threshold\_base} \cdot \left(1 + \beta \cdot (1 - Ratio_{energy})\right)$$

*Where $\beta$ (`ATP_HUNGER_BETA` = 0.5) is the hunger sensitivity coefficient. A starving neuron becomes deaf to weak noise.*

### 2.3. The Internal Clock (Phase Resonance)

Every tick, the neuron's phase shifts: $Phase = (Phase + 1) \pmod{256}$.

During `process_synapse` calculation, a linear approximation of resonance between Source and Target is computed:
$$Diff_{phase} = |Phase_{source} - Phase_{target}|$$
*(Adjusted for circular distance, max difference = 128)*

$$Multiplier_{phase} = 1.0 - \left(\frac{Diff_{phase}}{128}\right)$$
The incoming signal is multiplied by $Multiplier_{phase}$. Synchronized neurons transmit 100% of the signal, whereas those in complete anti-phase transmit 0%.

### 2.4. Precise STDP (Targeted Dendritic Learning)

Only the branches that contributed to the soma's activation are trained.

1. **Trace Update:** In `evaluate_branches`, the `local_trace` value of each branch decays exponentially every tick (e.g., `* 0.95`). If a branch produces an NMDA spike, its `local_trace` is set to 255.
2. **STDP Application:** In `process_synapse`, when calculating `delta_w` for LTP/LTD, the weight change is multiplied by the target branch's normalized trace:
    $$\Delta W_{final} = \Delta W \cdot \left(\frac{TargetBranch.local\_trace}{255}\right)$$
