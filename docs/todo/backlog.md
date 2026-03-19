# BACKLOG

## TODO

### 1. AoS vs SoA (Array of Structures)

Currently we use AoS (Array of Structures): data for one neuron (voltage, ATP, timers) is grouped together in memory, forming a solid 256-byte chunk.
SoA (Structure of Arrays) is when we place all voltages of all neurons into one long array, all ATP levels into another array, and so on.

For GPUs, SoA is paradise. When 32 threads (a warp) access memory for voltage, they grab one continuous chunk of memory (coalesced access) instead of jumping around the structure.
However, we explicitly fine-tuned the 256-byte alignment so that one neuron fits perfectly into an L1/L2 cache line. If we switch to SoA now, we would have to rewrite absolutely everything: C structures, FFI logic, rasterization. This is an enormous amount of work. I suggest dodging this item for now.

### 3. Raw CPU Fallbacks

The hacks with reinterpret_cast are a bit dirty, but they work purely as a stub on Mac when NVIDIA is missing. This does not affect simulation speed on the target hardware (Jetson Orin) at all. I suggest ignoring this for now, we can deal with aesthetics later.
