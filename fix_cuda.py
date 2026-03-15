import re

with open('cuda/base_neuron.cu', 'r') as f:
    code = f.read()

code = re.sub(
    r'    void tick_neuron_cpu\(BaseNeuron\* neuron, BaseNeuron\* all_neurons, int count, Environment env, uint32_t current_tick\) \{\n        \(void\)all_neurons;\n        \(void\)count;\n        tick_neuron_prepare_cpu\(neuron, env, current_tick\);\n        tick_neuron_spike_cpu\(neuron, env, current_tick\);\n    \}',
    '',
    code
)

with open('cuda/base_neuron.cu', 'w') as f:
    f.write(code)

