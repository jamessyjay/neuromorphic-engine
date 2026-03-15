import re

with open('tests/global_cycles_tests.rs', 'r') as f:
    code = f.read()

code = re.sub(
    r'    let motor = graph\.add_neuron\(NeuronType::Pyramidal\);\n    let mut env = default_env\(\);',
    r'    let motor = graph.add_neuron(NeuronType::Pyramidal);\n    let mut env = default_env();\n    graph.neurons[sensor as usize].column_id = 0;\n    graph.neurons[relay as usize].column_id = 1;\n    graph.neurons[motor as usize].column_id = 2;',
    code
)

with open('tests/global_cycles_tests.rs', 'w') as f:
    f.write(code)

