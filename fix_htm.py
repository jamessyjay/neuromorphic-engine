import re

with open('tests/htm_tests.rs', 'r') as f:
    code = f.read()

code = re.sub(
    r'    println!\("Predictive count: \{\}", predictive_count\);',
    r'    let test_v = ((graph.neurons[1].branches[4].voltage as f32) / 100.0) - 100.0; println!("Predictive count: {}, Branch V: {}, Status: {}", predictive_count, test_v, graph.neurons[1].status_flags);',
    code
)

with open('tests/htm_tests.rs', 'w') as f:
    f.write(code)

