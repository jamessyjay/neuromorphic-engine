import re

with open('tests/sparsity_tests.rs', 'r') as f:
    code = f.read()

# Since we updated WTA to have "Bursting" where all neurons spike if none are predictive,
# the test_sdr_activity_limit will now hit bursting (100% active) if we just blast all neurons with 50.0 without any predictive state.
# To properly test SDR, we should give them different voltages and make sure WTA limits it, OR make them predictive.
# Wait, bursting occurs if *no* predictive neuron crosses threshold.
# Let's set the first neuron in each minicolumn to be predictive so WTA properly suppresses the rest.

code = re.sub(
    r'    // Blast them all with massive excitation \(noise\)\n    for i in 0\.\.1000 \{\n        graph\.neurons\[i\]\.current_voltage = 50\.0 \+ \(i % 20\) as f32;\n    \}',
    r'    // Blast them all with massive excitation (noise)\n    for i in 0..1000 {\n        graph.neurons[i].current_voltage = 50.0 + (i % 20) as f32;\n        if i % 10 == 0 {\n            // Make one neuron per minicolumn predictive so we trigger WTA instead of Bursting\n            graph.neurons[i].status_flags |= 2; // FLAG_IS_PREDICTIVE\n        }\n    }',
    code
)

with open('tests/sparsity_tests.rs', 'w') as f:
    f.write(code)

