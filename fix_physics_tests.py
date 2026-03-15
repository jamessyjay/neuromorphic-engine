import re

with open('tests/neuron_physics_tests.rs', 'r') as f:
    code = f.read()

code = re.sub(r'tick_neuron_cpu', 'tick_neuron_prepare_cpu, tick_neuron_spike_cpu', code)

with open('tests/neuron_physics_tests.rs', 'w') as f:
    f.write(code)

