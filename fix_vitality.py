import re

with open('tests/vitality_tests.rs', 'r') as f:
    code = f.read()

code = re.sub(
    r'    fn tick_neuron_cpu\(neuron: \*mut BaseNeuron, all_neurons: \*mut BaseNeuron, count: std::ffi::c_int, env: Environment, current_tick: u32\);',
    r'    fn tick_neuron_prepare_cpu(neuron: *mut BaseNeuron, env: Environment, current_tick: u32);\n    fn tick_neuron_spike_cpu(neuron: *mut BaseNeuron, env: Environment, current_tick: u32);',
    code
)

code = re.sub(
    r'unsafe \{ tick_neuron_cpu\(&mut neuron, std::ptr::null_mut\(\), 0, env\.clone\(\), (.*?)\); \}',
    r'unsafe { tick_neuron_prepare_cpu(&mut neuron, env.clone(), \1); tick_neuron_spike_cpu(&mut neuron, env.clone(), \1); }',
    code
)

with open('tests/vitality_tests.rs', 'w') as f:
    f.write(code)

