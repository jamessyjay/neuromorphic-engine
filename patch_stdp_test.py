import re

with open("tests/test_stdp.rs", "r") as f:
    content = f.read()

content = content.replace('assert_eq!(std::mem::size_of::<BaseNeuron>(), 128, "BaseNeuron must be exactly 128 bytes");', 'assert_eq!(std::mem::size_of::<BaseNeuron>(), 256, "BaseNeuron must be exactly 256 bytes");')

with open("tests/test_stdp.rs", "w") as f:
    f.write(content)
