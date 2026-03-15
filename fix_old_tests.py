import re

with open('tests/columnar_tests.rs', 'r') as f:
    code = f.read()

code = re.sub(
    r'    // Force spike on Neuron 1\n    graph\.force_sync_gpu\(\);\n    graph\.neurons\[1\]\.current_voltage = 50\.0;',
    r'    // Force spike on Neuron 1 and make it predictive\n    graph.force_sync_gpu();\n    graph.neurons[1].current_voltage = 50.0;\n    graph.neurons[1].status_flags |= constants::FLAG_IS_PREDICTIVE;\n    graph.neurons[1].predictive_ticks = 10;',
    code
)

code = re.sub(
    r'    // Force spike on Neuron A\n    graph\.force_sync_gpu\(\);\n    graph\.neurons\[0\]\.current_voltage = 50\.0; // Above threshold to trigger spike in phase 1\n    \n    // Run tick 1\n    graph\.tick\(1, env\);\n    \n    // Neuron 0 should have spiked in tick 1\n    assert_eq!\(graph\.neurons\[0\]\.last_spike_tick, 1, "Neuron A should have spiked"\);\n    \n    // Run tick 2 to process suppression\n    graph\.neurons\[0\]\.current_voltage = 50\.0;\n    graph\.tick\(2, env\.clone\(\)\);\n    \n    // Check Neuron 1 \(Neuron B\) for FLAG_IS_INHIBITED right after the tick where Neuron A spiked\n    assert_eq!\(\(graph\.neurons\[1\]\.status_flags & constants::FLAG_IS_INHIBITED\) > 0, true, "Neuron B must have FLAG_IS_INHIBITED set due to WTA suppression from Neuron A\."\);',
    r'''    // Force spike on Neuron A and make it predictive to trigger WTA instead of Bursting
    graph.force_sync_gpu();
    graph.neurons[0].current_voltage = 50.0; // Above threshold to trigger spike in phase 1
    graph.neurons[0].status_flags |= constants::FLAG_IS_PREDICTIVE;
    graph.neurons[0].predictive_ticks = 10;
    
    // Run tick 1
    graph.tick(1, env);
    
    // Neuron 0 should have spiked in tick 1
    assert_eq!(graph.neurons[0].last_spike_tick, 1, "Neuron A should have spiked");
    
    // Check Neuron 1 (Neuron B) for FLAG_IS_INHIBITED right after the tick where Neuron A spiked
    assert_eq!((graph.neurons[1].status_flags & constants::FLAG_IS_INHIBITED) > 0, true, "Neuron B must have FLAG_IS_INHIBITED set due to WTA suppression from Neuron A.");''',
    code
)

with open('tests/columnar_tests.rs', 'w') as f:
    f.write(code)

