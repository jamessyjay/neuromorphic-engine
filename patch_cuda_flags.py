import re

with open("cuda/base_neuron.cu", "r") as f:
    content = f.read()

# Add flag clearing logic to tick_neurons_kernel if it doesn't exist
pattern = r'(__global__ void tick_neurons_kernel[\s\S]*?if \(!neurons\[idx\]\.is_alive\)\s*\{)'
replacement = r'__global__ void tick_neurons_kernel(BaseNeuron* neurons, int count, Environment env, uint32_t current_tick) {\n    int idx = blockIdx.x * blockDim.x + threadIdx.x;\n    if (idx >= count) return;\n    \n    // WTA Flag Reset: Clear inhibition status at start of tick\n    neurons[idx].status_flags &= ~(1 << 0); // FLAG_IS_INHIBITED\n    \n    // Trace Cleanup: Ensure spike_trace is properly decayed or zeroed\n\n    if (!neurons[idx].is_alive) {'

# Let's replace precisely
content = re.sub(r'__global__ void tick_neurons_kernel\(BaseNeuron\* neurons, int count, Environment env, uint32_t current_tick\) \{\n    int idx = blockIdx\.x \* blockDim\.x \+ threadIdx\.x;\n    if \(idx >= count\) return;\n    \n    // Trace Cleanup: Ensure spike_trace is properly decayed or zeroed\n\n    if \(!neurons\[idx\]\.is_alive\) \{', replacement, content)

with open("cuda/base_neuron.cu", "w") as f:
    f.write(content)
