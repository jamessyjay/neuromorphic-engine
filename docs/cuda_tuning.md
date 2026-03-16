# The Path to Olympus: CUDA Tuning in Neuromorphic Engine

This document details exactly how we squeezed every ounce of performance out of the GPU and broke through the TPS (Ticks Per Second) ceiling for simulating 10,000+ neurons and hundreds of thousands of synapses by eliminating architectural bottlenecks.

---

## 1. The Problem: $O(N^2)$ and the Death of WTA

Initially, the **Winner-Takes-All (WTA)** (lateral inhibition in minicolumns) implementation was built in a naive way. Every neuron in the `__global__` kernel ran a nested loop over all other neurons to check who was in its minicolumn and had a higher voltage.
This resulted in an $O(N^2)$ complexity.
With 1000 neurons it worked tolerably well, but at 10,000 neurons the GPU ground to a halt. The TPS dropped from 2500 down to a laughable **212 TPS**.

---

## 2. The Solution: Global "Bulletin Board" $O(1)$

To kill the nested loops, we implemented a "Bulletin Board" pattern.
Instead of searching for neighbors, each neuron simply "shouts" its result into a global array.

### 2.1 Atomic Meatgrinder and Bit-Packing

We couldn't use a `struct` for the board because CUDA only supports `atomicCAS` operations for primitives (32-bit or 64-bit int).
Using `memcpy` inside concurrent threads is suicide (it caused Data Races).

**What we did:**
We packed 3 values (voltage, neuron ID, and tick) into a single 64-bit `unsigned long long int`:

- Lower 32 bits `[0..31]`: `float` voltage (via the `__float_as_uint` intrinsic).
- Next 22 bits `[32..53]`: Neuron ID.
- Upper 10 bits `[54..63]`: Current tick mask (`current_tick % 1024`).

**The magic of `atomicCAS`:**
Neurons use a `do-while` loop to try and write their voltage to the board. If their voltage is higher (or if the voltage is equal but the ID is lower), they overwrite the leader. This is $O(1)$ complexity.

*TPS jumped right back to 2300-2500 on 10k neurons.*

---

## 3. Mathematical Hygiene: Size Matters

Simultaneously, we discovered that the `BaseNeuron` struct had bloated to weird sizes due to compiler padding.
In CUDA, it's critically important that structs fit perfectly into Cache Lines. The ideal size for us is strictly **256 bytes**.

**What we did:**

1. Threw away unnecessary reserved fields.
2. Moved `calcium_trace` to be immediately after `calcium_level` (improving spatial locality).
3. Added manual `_pad_block` arrays of type `uint8_t` to fill the "holes" between variables.
4. Added `static_assert(sizeof(BaseNeuron) == 256, "...");` so no one in the future breaks this balance.

Memory became dense, and Cache Misses plummeted to near zero.

---

## 4. The Fastpath: Killing Orchestration Overhead (Kernel Fusion)

Even after solving the $O(N^2)$ problem, we noticed we were losing about 1000 TPS just to the overhead of the memory bus and the GPU task dispatcher.

### 4.1 Kernel Fusion

We had two separate kernels: `tick_neurons_prepare_kernel` (metabolism) and `wta_and_burst_vote_kernel` (WTA voting).
This meant the GPU was:

1. Loading the 256-byte neuron from VRAM into thread registers.
2. Calculating metabolism.
3. Storing the 256 bytes back to VRAM.
4. *Loading* the exact same 256 bytes *again* for the voting kernel!

We fused them into one monstrous kernel: **`tick_neurons_prep_and_vote_kernel`**.
Now the neuron wakes up, consumes ATP, updates calcium, gets thermal noise, and **immediately**, without leaving the registers, goes to vote on the bulletin board.
The Memory Bandwidth savings are colossal.

### 4.2 Exorcising `cudaDeviceSynchronize`

Between kernel launches, the host (Rust CPU) was calling `cudaDeviceSynchronize()`.
But kernels launched in the same (default/zero) CUDA stream are hardware-guaranteed to execute **strictly sequentially**. The synchronizations were only forcing the CPU to sit idle while the PCIe bus passed empty acknowledgments back and forth.

We **removed all intermediate synchronization barriers** in the `launch_tick_neurons` function, leaving only one at the very end (to return control back to Rust).

---

## 5. Summary: Pure Power

After implementing all these techniques (`O(1)` WTA, Bit-Packing, 256-byte Alignment, Kernel Fusion, Zero-Sync Pipeline), we achieved:

1. **Perfect Scalability**: The TPS drop-off graph as the network grows became strictly linear, not parabolic.
2. **Breaking the Barrier**: On real hardware, we broke through the **3000 TPS** ceiling for 10,000 neurons and 100,000 synapses.
3. **ETALON Quality**: The code became mathematically hygienic, with no "dirty" `memcpy` hacks inside atomic sections.

We forced the GPU to work exactly as it was designed to: grinding through arrays of math without stopping or looking back at the CPU.
