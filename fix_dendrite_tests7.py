import re

with open("tests/dendrite_tests.rs", "r") as f:
    content = f.read()

# Let's print the actual values to see what they are!
content = content.replace('assert_eq!(graph.neurons[2].predictive_ticks, 19, "Distal branch must trigger Predictive State");', 'println!("Predictive Ticks: {}, Target Resting: {}, Current V: {}", graph.neurons[2].predictive_ticks, target_resting, graph.neurons[2].current_voltage);\n    assert_eq!(graph.neurons[2].predictive_ticks, 19, "Distal branch must trigger Predictive State");')

with open("tests/dendrite_tests.rs", "w") as f:
    f.write(content)
