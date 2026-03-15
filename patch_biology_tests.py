import re

with open("tests/biology_tests.rs", "r") as f:
    content = f.read()

# Update BaseNeuron initializations in biology tests that use the old 8 branches or old structure
content = re.sub(r'branches: \[Compartment \{\s*branch_type: 0,\s*refractory_timer: 0,\s*voltage: 0,\s*nmda_threshold: 0,\s*local_trace: 0,\s*_pad: 0\s*\}; 8\],', r'column_id: 0, minicolumn_id: 0, layer_id: 0, status_flags: 0, _pad_block1: [0; 32], ref_coord_x: 0.0, ref_coord_y: 0.0, _pad_block2: [0; 16], branches: [Compartment { branch_type: 0, refractory_timer: 0, voltage: 0, nmda_threshold: 0, local_trace: 0, _pad: 0 }; 16],', content)

with open("tests/biology_tests.rs", "w") as f:
    f.write(content)
