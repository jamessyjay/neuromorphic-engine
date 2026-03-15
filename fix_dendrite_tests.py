import re

with open("tests/dendrite_tests.rs", "r") as f:
    content = f.read()

# Ah! The branches are initialized with `branch_type = 0` (empty) and `nmda_threshold = 0.0`.
# In `evaluate_branches`: `if (b->branch_type == 0) continue;`
# So the test's branches are ignored!
# We need to manually set the `branch_type` and `nmda_threshold` in the test before running.
# Proximal is branch 2. Distal is branch 6.

replacement = """
    // Initialize branch properties so they are not ignored
    graph.neurons[2].branches[2].branch_type = 2; // Trunk/Proximal
    graph.neurons[2].branches[2].nmda_threshold = crate::engine::ffi::pack_voltage(10.0);
    graph.neurons[2].branches[6].branch_type = 3; // Tuft/Distal
    graph.neurons[2].branches[6].nmda_threshold = crate::engine::ffi::pack_voltage(10.0);

    let target_resting = graph.neurons[2].resting_voltage;

    // Phase 1: Fire Distal (Branch 6)
"""

content = content.replace("let target_resting = graph.neurons[2].resting_voltage;\n\n    // Phase 1: Fire Distal (Branch 6)", replacement)

with open("tests/dendrite_tests.rs", "w") as f:
    f.write(content)
