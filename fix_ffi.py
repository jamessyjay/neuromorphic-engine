import re

with open('src/engine/ffi.rs', 'r') as f:
    code = f.read()

code = re.sub(
    r'    pub fn tick_neuron_cpu\(neuron: \*mut BaseNeuron, all_neurons: \*mut BaseNeuron, count: c_int, env: Environment, current_tick: u32\);',
    '''    pub fn tick_neuron_prepare_cpu(neuron: *mut BaseNeuron, env: Environment, current_tick: u32);
    pub fn tick_neuron_spike_cpu(neuron: *mut BaseNeuron, env: Environment, current_tick: u32);''',
    code
)

with open('src/engine/ffi.rs', 'w') as f:
    f.write(code)
