import re

with open('tests/htm_tests.rs', 'r') as f:
    code = f.read()

code = re.sub(
    r'use neuromorphic_engine::engine::ffi::\{NeuronType, Environment, Compartment, Synapse\};',
    r'use neuromorphic_engine::engine::ffi::{NeuronType, Environment};',
    code
)

with open('tests/htm_tests.rs', 'w') as f:
    f.write(code)

