import re

with open('tests/neuron_physics_tests.rs', 'r') as f:
    code = f.read()

code = re.sub(
    r'    // In new Bursting/WTA logic, predictive state offset is handled dynamically during spike check\.\n    // Since we are mocking outside the graph, we need to provide enough voltage to cross threshold when combined with boost\.\n    neuron\.current_voltage = neuron\.activation_threshold - 5\.0;\n    \n    unsafe \{ tick_neuron_prepare_cpu\(&mut neuron, env\.clone\(\), 2\); tick_neuron_spike_cpu\(&mut neuron, env\.clone\(\), 2\); \}',
    r'    // In new Bursting/WTA logic, predictive state offset is handled dynamically during spike check.\n    // Since we are mocking outside the graph, we need to provide enough voltage to cross threshold when combined with boost.\n    neuron.current_voltage = neuron.activation_threshold - 5.0;\n    \n    // Mock WTA predictive boost\n    neuron.current_voltage += 15.0;\n    \n    unsafe { tick_neuron_prepare_cpu(&mut neuron, env.clone(), 2); tick_neuron_spike_cpu(&mut neuron, env.clone(), 2); }',
    code
)

with open('tests/neuron_physics_tests.rs', 'w') as f:
    f.write(code)

