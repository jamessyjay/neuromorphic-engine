import re

with open('tests/neuron_physics_tests.rs', 'r') as f:
    code = f.read()

code = re.sub(
    r'tick_neuron_cpu\((.*?), std::ptr::null_mut\(\), 0, (.*?), (.*?)\);',
    r'tick_neuron_prepare_cpu(\1, \2, \3); tick_neuron_spike_cpu(\1, \2, \3);',
    code
)

with open('tests/neuron_physics_tests.rs', 'w') as f:
    f.write(code)

