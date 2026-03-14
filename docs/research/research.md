
# Neuromorphic Engine: High-Performance GPU SNN
**Status**: Research & Concept phase.

## 1. Biologically Plausible Neurons (CUDA)
The core of the simulation runs entirely on the GPU, executing complex biological models instead of simple perceptrons.

### **The Hodgkin-Huxley (HH) Biophysical Model**
To achieve biological accuracy at the micro-level, project specifications mandate the integration of the Hodgkin-Huxley model [2]. Unlike the phenomenological LIF model, the HH model meticulously replicates the kinetics of ion channels through a system of nonlinear ordinary differential equations describing sodium (![][image14]), potassium (![][image15]) currents, and the leak current. The central current balance equation takes the form:
![][image16]

The dynamics of each activation and inactivation gating variable ![][image17] are described by a first-order kinetic equation:
![][image18]

The transition rate functions ![][image19] and ![][image20] depend exponentially on the current membrane potential, which generates critical computational challenges. Implementing an HH-neuron in a SIMT (Single Instruction, Multiple Threads) architecture creates extreme pressure on the Special Function Units (SFU) inside the SM, as each thread is forced to calculate multiple transcendental functions (expf) on every iteration. Furthermore, the high stiffness of the HH system of differential equations makes the forward Euler method unstable. To prevent numerical divergence, it requires implementing the Exponential Euler method or higher-order Runge-Kutta methods, which entails an exponential increase in register usage (register spilling) and a catastrophic drop in parallelism.

| Biophysical Characteristic | LIF Model (Leaky Integrate-and-Fire) | HH Model (Hodgkin-Huxley) | Impact on GPU Microarchitecture |
| :---- | :---- | :---- | :---- |
| **Complexity (FLOPs / iteration)** | Low (~10-15 operations) | High (>150 operations, including exponentials) | HH mode is strictly compute-bound. |
| **State Size (Memory Footprint)** | ~16-24 Bytes (![][image1], refractory timer, current) | ~48-64 Bytes (![][image1], ![][image21], ![][image22], ![][image23], conductances) | HH mode increases load on memory controllers, causing cache misses. |
| **Numerical Method Stability** | Euler method is sufficient | RK4 / Exponential Euler required | Branch divergence and higher register pressure in CUDA kernels. |
| **Maximum Simulation Density** | Tens of millions of nodes per GPU | Hundreds of thousands of nodes per GPU | Requires aggressive graph clustering and partitioning for HH. |

## 2. Architectural Graph and Data Flow Topology
The architectural semantics of neuromorphic-engine are built around a strictly deterministic and unidirectional data processing pipeline: Stimulus -> Graph -> Neuron -> Synapse -> Telemetry [3]. This architectural graph dictates the lifecycle of every simulation step, managing system state and minimizing costly synchronizations between the Host (CPU) and Device (GPU).

### **Component Hierarchy and Topology Serialization**
The Stimulus module acts as the source of external excitation needed to bring the network out of its resting state. In the context of biological modeling, stimuli are implemented via independent Poisson process generators (Poisson Spike Trains) that mimic background cortical activity, or through deterministic current injection patterns. The computational challenge at this stage lies in generating high-quality pseudorandom numbers. If generation happens on the CPU followed by transferring stimulus matrices to the GPU via PCIe on every time step, the bus bandwidth becomes an absolute bottleneck for the entire system.

The Graph module serves as the central hub, encapsulating structural connections between neurons [3]. Traditional object-oriented graph representations using pointers and adjacency lists are incompatible with GPU architecture due to chaotic memory access patterns. In high-performance SNN engines, the network must be mathematically serialized into Compressed Sparse Row (CSR) or Compressed Sparse Column (CSC) format. In CSR format, the entire synaptic weight matrix is compressed into three flat 1D arrays: weights array, column index array (postsynaptic targets), and row pointer array. This structure ensures that when iterating over outgoing synapses of a spiking neuron, CUDA threads read data coalesced (in continuous blocks), fully utilizing the 128-bit global memory transaction width.

The Neuron kernel executes the biophysical integration of states [2]. After topological serialization, neuron states are represented as SoA (Structure of Arrays). The CUDA kernel computes the state update (![][image24], refractory period, neurotransmitter buffer) for each node simultaneously. A critical optimization at this stage is avoiding warp divergence.

The Synapse kernel handles spike propagation and plasticity. This module realizes STDP (Spike-Timing-Dependent Plasticity) logic [3], adjusting connection weights based on the correlation of pre- and postsynaptic spikes. STDP implementation requires historical memory of spike timings, dramatically increasing the memory footprint.

Finally, the Telemetry module aggregates network state for analysis and visualization. To prevent pipeline stalls, telemetry must be gathered asynchronously using CUDA Streams and Event APIs, avoiding implicit host synchronization.

### **Concurrency Constraints and Race Conditions**
The fundamental problem in parallel SNN simulation lies in writing synaptic currents. When multiple presynaptic neurons spike simultaneously and project to the same postsynaptic neuron, multiple threads attempt to update the target neuron's current buffer concurrently. In CUDA, this creates a classic Race Condition.

To resolve this, the engine must employ atomic operations (`atomicAdd`). However, massive atomic collisions on highly connected hub neurons (e.g., in scale-free networks) serialize execution, drastically degrading performance. Advanced implementations utilize a "Warp-Level Reduction" strategy: threads in a warp that share the same target address first sum their currents in local registers using `__shfl_down_sync()` instructions or through Shared Memory. Only the final aggregated current is atomically written to the target neuron's global memory by a single leader thread (Lane 0).

### **I/O Bottlenecks and Asynchronous Writes**
High-speed simulation generates gigabytes of telemetry per second. If the CPU synchronously waits to copy the buffer from the GPU and then blocks on OS calls to write this buffer to an SSD, the GPU will sit idle.

The solution involves a Double Buffering pattern coupled with CUDA Streams. While the GPU computes time interval N in Stream 1, the CPU initiates an asynchronous copy of interval N-1 results from Stream 0. Upon receiving a data ready signal from the driver (`cudaEventQuery`), a dedicated OS asynchronous thread (using the Tokio framework in Rust) flushes the array to disk in a compressed binary format (HDF5 or Apache Parquet), freeing the CPU from I/O blocks.

## 3. Integration of Multi-GPU Scaling
Given the mathematical and topological ambitions of the project [2], a full-scale simulation of a mammalian brain (e.g., rhesus macaque or human) is physically impossible within the VRAM of a single modern accelerator (like the NVIDIA H100 with 80GB VRAM). The development of a **Multi-GPU cluster scaling interface** is essential.

*Architectural Implementation:* Integration of the NVIDIA Collective Communications Library (NCCL) via C-bindings into the Rust subsystem. The neuromorphic-engine computational graph must be partitioned into local subgraphs. This requires implementing Graph Partitioning algorithms, such as METIS or the Louvain method, to minimize the edge cut between clusters. Neurons whose axons cross the physical boundaries of GPU nodes are marked as Boundary Nodes. Spikes from these neurons are packed into ultra-dense bit vectors and transmitted directly via NVLink interconnects or PCIe bus in Peer-to-Peer (P2P) mode, completely bypassing the host CPU and RAM.

## 4. Context Prompt for AI-Assisted Scaling
To ensure seamless onboarding of LLMs (Large Language Models) as code-generation copilots or to brief new Senior developers, the following high-density system prompt is provided. This prompt encapsulates the entire technical context, the biophysical nature of the project, and imposes strict technological constraints to preclude the generation of suboptimal or unsafe code.

Implementing this context guarantees that any AI system enlisted for refactoring or extending the codebase will inherently align with the strict laws of hardware efficiency and maintain the delicate balance between the safety of the Rust environment and the extreme performance of CUDA accelerators.
