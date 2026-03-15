import re

with open('tests/columnar_tests.rs', 'r') as f:
    code = f.read()

code = re.sub(
    r'    // Check Neuron 1 \(Neuron B\) for FLAG_IS_INHIBITED right after the tick where Neuron A spiked\n    assert_eq!\(\(graph\.neurons\[1\]\.status_flags & constants::FLAG_IS_INHIBITED\) > 0, true, "Neuron B must have FLAG_IS_INHIBITED set due to WTA suppression from Neuron A\."\);',
    r'    // Run tick 2 to process suppression\n    graph.neurons[0].current_voltage = 50.0;\n    graph.tick(2, env.clone());\n    \n    // Check Neuron 1 (Neuron B) for FLAG_IS_INHIBITED right after the tick where Neuron A spiked\n    assert_eq!((graph.neurons[1].status_flags & constants::FLAG_IS_INHIBITED) > 0, true, "Neuron B must have FLAG_IS_INHIBITED set due to WTA suppression from Neuron A.");',
    code
)

with open('tests/columnar_tests.rs', 'w') as f:
    f.write(code)

