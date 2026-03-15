import re
import os

for file in ['tests/vitality_tests.rs', 'tests/neuron_physics_tests.rs']:
    with open(file, 'r') as f:
        code = f.read()
        
    code = re.sub(
        r'unsafe \{ crate::engine::ffi::tick_neuron_cpu\((.*?), std::ptr::null_mut\(\), 0, (.*?), (.*?)\); \}',
        r'unsafe { crate::engine::ffi::tick_neuron_prepare_cpu(\1, \2, \3); crate::engine::ffi::tick_neuron_spike_cpu(\1, \2, \3); }',
        code
    )
    
    with open(file, 'w') as f:
        f.write(code)

