import re

with open('tests/neuron_physics_tests.rs', 'r') as f:
    code = f.read()

code = re.sub(
    r'use neuromorphic_engine::engine::ffi::\{tick_neuron_cpu, tick_synapse_cpu, init_neuron\};',
    r'use neuromorphic_engine::engine::ffi::{tick_neuron_prepare_cpu, tick_neuron_spike_cpu, tick_synapse_cpu, init_neuron};',
    code
)

code = re.sub(
    r'unsafe \{ tick_neuron_cpu\((.*?)\); \}',
    r'unsafe { tick_neuron_prepare_cpu(\1); tick_neuron_spike_cpu(\1); }',
    code
)

with open('tests/neuron_physics_tests.rs', 'w') as f:
    f.write(code)

