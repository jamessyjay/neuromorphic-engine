# Bug Fixes & Optimization Report (Phase 34)

## 1. Memory Geometry Optimization (Alignment & Cache Lines)

### The Problem

The `Compartment` struct was implicitly taking up 12 bytes due to C++ alignment rules (1 byte `branch_type` + 3 bytes padding + 4 bytes `voltage` + 4 bytes `nmda_threshold`). This caused the 8-branch array to take 96 bytes, pushing the total `BaseNeuron` size to ~160 bytes. This size did not align with the 64-byte GPU cache line architecture, causing the GPU to fetch 3 cache lines per neuron, significantly degrading massive parallel performance.

### The Solution

- **Fixed-Point Compression:** Compressed `voltage` and `nmda_threshold` from 32-bit `float` to 16-bit `uint16_t` using a custom fixed-point macro (`pack_voltage`/`unpack_voltage`). The macro maps the biological range (-100.0mV to +100.0mV) to a 0-65535 integer range, preserving 2 decimal places of precision.
- **Compartment Packing:** The `Compartment` struct is now exactly 8 bytes: `branch_type` (1) + `refractory_timer` (1) + `voltage` (2) + `nmda_threshold` (2) + `_padding` (2).
- **BaseNeuron Compression:** We compressed several fields in the base struct to shrink the main footprint:
  - `spikes_in_last_window` -> `uint16_t`
  - `metabolic_wear` -> packed to `uint16_t` (0.0 to 1.0 mapped to 0-65535)
  - `x, y, z` coordinates -> packed to `uint16_t` (0.0 to 100.0 mapped to 0-65535)
  - Changed `NeuronType` enum from 4 bytes to `uint8_t`.
- **Strict Alignment:** Added `alignas(64)` in C++ and `#[repr(align(64))]` in Rust.
- **Result:** `BaseNeuron` is now exactly **128 bytes** (Base fields: 64 bytes + Branches: 64 bytes). It perfectly fits into 2 cache lines, completely eliminating memory latency overhead.

---

## 2. Resolving "Digital Deafness" (The Passive Leak Gap)

### A. The Problem

In the original `evaluate_branches` logic, synaptic voltage in dendrites only affected the Soma if an NMDA-spike was triggered (`voltage > nmda_threshold`). If a branch accumulated 4.9mV against a 5.0mV threshold, the Soma received 0. This destroyed the analog gradient of biological neurons, turning branches into rigid digital logic gates.

### B. The Solution

- Implemented **Passive Flow**. Every tick, a small percentage of a branch's voltage leaks into the Soma, regardless of whether a spike occurs.
- Ensured **Charge Conservation**: The voltage leaked into the Soma is subtracted from the branch's local voltage (`b->voltage -= passive_leak`).
- Now, sub-threshold signals slowly accumulate in the center, allowing the neuron to "feel" weak but sustained inputs.

---

## 3. Fixing the "Eternal High" Bug (Voltage Reset)

### A.The Problem

When a branch fired an NMDA-spike, its voltage was simply set to `0.0f`. However, if the synaptic buffer still had neurotransmitters, they would immediately flow back into the branch on the very next tick, potentially causing an immediate secondary spike and creating an infinite feedback loop of hyper-activity.

### B.The Solution

- Added a `uint8_t refractory_timer` to the `Compartment` struct.
- When an NMDA spike occurs, `refractory_timer` is set to 3 ticks.
- While the timer is active (`> 0`), the branch's voltage is hard-locked to `0.0f` and the timer decrements. Any incoming chemical signals during this phase are effectively ignored by the branch (the chemistry is consumed, but no voltage is generated).

---

## 4. Rust and C++ FFI Synchronization

### 4.1. The Problem

The Rust implementation of `BaseNeuron` and `Compartment` did not have explicit padding or fixed-point types, and lacked the `#[repr(align(64))]` directive. This meant the Rust compiler could reorder fields or misalign memory, causing pointers passed to CUDA to read garbage data (e.g., reading a float where a uint8_t should be).

### 4.2. The Solution

- Enforced strict 1:1 mapping between `src/ffi.rs` and `cuda/base_neuron.h`.
- Used `#[repr(C)]` on all structs.
- Added explicit dummy padding fields (`_pad1`, `_pad2`) in Rust to match C++ alignment.
- Converted Rust fields to `u16` where fixed-point packing was introduced in C++.
- Added `#[repr(align(64))]` to `BaseNeuron` in Rust.

---

## 5. Validation Test (`--passive-leak-test`)

Created a dedicated execution mode to mathematically prove the fixes:

1. Injected a 10.0mV signal into a branch with a 50.0mV threshold. The test verified that the Soma's voltage increased above its resting potential (-70.0mV) on the very next tick, confirming the **Passive Leak**.
2. Injected a 60.0mV signal to force an NMDA spike. The test verified the `refractory_timer` was set to 3.
3. Injected a secondary signal while the timer was active. The test verified the branch voltage remained `0.0mV` and the timer decremented, confirming the **Eternal High** bug is patched.
