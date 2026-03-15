import re
import os

for file in ['tests/vitality_tests.rs', 'tests/neuron_physics_tests.rs']:
    with open(file, 'r') as f:
        code = f.read()
        
    code = re.sub(
        r'unsafe \{ \[ \{\n\s*tick_neuron_prepare_cpu, tick_neuron_spike_cpu\((.*?), std::ptr::null_mut\(\), 0, (.*?), (.*?)\);\n\s*\]',
        r'unsafe { tick_neuron_prepare_cpu(\1, \2, \3); tick_neuron_spike_cpu(\1, \2, \3); }',
        code
    )
    
    code = re.sub(
        r'unsafe \{\s*tick_neuron_prepare_cpu, tick_neuron_spike_cpu\((.*?), std::ptr::null_mut\(\), 0, (.*?), (.*?)\);\n\s*\}',
        r'unsafe { tick_neuron_prepare_cpu(\1, \2, \3); tick_neuron_spike_cpu(\1, \2, \3); }',
        code
    )
    
    code = re.sub(
        r'tick_neuron_prepare_cpu, tick_neuron_spike_cpu\((.*?), std::ptr::null_mut\(\), 0, (.*?), (.*?)\);',
        r'tick_neuron_prepare_cpu(\1, \2, \3); tick_neuron_spike_cpu(\1, \2, \3);',
        code
    )

    with open(file, 'w') as f:
        f.write(code)

