import re

with open('src/engine/ffi.rs', 'r') as f:
    code = f.read()

code = re.sub(
    r'    pub fn tick_synapse_cpu\(synapse: \*mut Synapse, neurons: \*mut BaseNeuron, env: Environment, current_tick: u32\);\n    pub fn tick_synapse_cpu\(synapse: \*mut Synapse, neurons: \*mut BaseNeuron, env: Environment, current_tick: u32\);',
    r'    pub fn tick_synapse_cpu(synapse: *mut Synapse, neurons: *mut BaseNeuron, env: Environment, current_tick: u32);',
    code
)

with open('src/engine/ffi.rs', 'w') as f:
    f.write(code)

