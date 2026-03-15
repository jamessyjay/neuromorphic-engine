import re

with open("src/network/graph.rs", "r") as f:
    content = f.read()

# Update BaseNeuron initialization
pattern = r'(let mut neuron = BaseNeuron \{[\s\S]*?)(branches: \[crate::engine::ffi::Compartment \{[\s\S]*?\}; 8\],)'
replacement = r'\g<1>column_id: 0,\n            minicolumn_id: 0,\n            layer_id: 0,\n            status_flags: 0,\n            _pad_block1: [0; 32],\n            ref_coord_x: 0.0,\n            ref_coord_y: 0.0,\n            _pad_block2: [0; 16],\n            branches: [crate::engine::ffi::Compartment {\n                branch_type: 0,\n                refractory_timer: 0,\n                voltage: 0,\n                nmda_threshold: 0,\n                local_trace: 0,\n                _pad: 0,\n            }; 16],'

content = re.sub(pattern, replacement, content)

with open("src/network/graph.rs", "w") as f:
    f.write(content)

