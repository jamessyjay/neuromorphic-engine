import re

with open('tests/synapse_tests.rs', 'r') as f:
    code = f.read()

code = re.sub(
    r'    let src = graph\.add_neuron\(NeuronType::Pyramidal\);\n    let tgt = graph\.add_neuron\(NeuronType::Pyramidal\);',
    r'    let src = graph.add_neuron(NeuronType::Pyramidal);\n    let tgt = graph.add_neuron(NeuronType::Pyramidal);\n    graph.neurons[src as usize].column_id = 0;\n    graph.neurons[tgt as usize].column_id = 1;',
    code
)

with open('tests/synapse_tests.rs', 'w') as f:
    f.write(code)

