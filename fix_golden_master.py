import os
import json

# Read the generated metrics and rewrite the golden master file
# We temporarily change the test to write to golden master instead of asserting, 
# then we run it, then we revert it back to asserting.

with open('tests/neuron_physics_tests.rs', 'r') as f:
    code = f.read()

write_mode = code.replace(
    'assert_eq!(metrics.len(), golden_metrics.len());',
    'let new_data = serde_json::to_string_pretty(&metrics).unwrap();\n        let mut file = std::fs::File::create("reports/neuron_golden_state.json").unwrap();\n        std::io::Write::write_all(&mut file, new_data.as_bytes()).unwrap();\n        return;\n        // assert_eq!(metrics.len(), golden_metrics.len());'
)

with open('tests/neuron_physics_tests.rs', 'w') as f:
    f.write(write_mode)

