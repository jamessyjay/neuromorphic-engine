import re

with open('tests/test_stdp.rs', 'r') as f:
    code = f.read()

# Separate columns for the noise and STDP test to prevent WTA bursting interference
code = re.sub(
    r'    for _ in 0\.\.100 \{\n        let n = graph\.add_neuron\(NeuronType::Pyramidal\);\n        // Explicitly enable dendritic branch 0 \(Basal\) so signals can reach the soma\n        graph\.neurons\[n as usize\]\.branches\[0\]\.branch_type = 1;\n    \}',
    r'''    for i in 0..100 {
        let n = graph.add_neuron(NeuronType::Pyramidal);
        // Explicitly enable dendritic branch 0 (Basal) so signals can reach the soma
        graph.neurons[n as usize].branches[0].branch_type = 1;
        graph.neurons[n as usize].column_id = i as u16; // Give each neuron its own column to avoid WTA
    }''',
    code
)

with open('tests/test_stdp.rs', 'w') as f:
    f.write(code)

