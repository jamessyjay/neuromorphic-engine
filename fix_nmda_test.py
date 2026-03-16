import re

with open('tests/dendrite_tests.rs', 'r') as f:
    code = f.read()

code = re.sub(
    r'graph\.neurons\[id as usize\]\.branches\[0\]\.voltage = pack_voltage\(25\.0\);',
    r'graph.neurons[id as usize].branches[0].voltage = pack_voltage(80.0);',
    code
)

with open('tests/dendrite_tests.rs', 'w') as f:
    f.write(code)

