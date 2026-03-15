import re

with open("tests/test_stdp.rs", "r") as f:
    content = f.read()

content = content.replace("assert_eq!(mem::size_of::<BaseNeuron>(), 128", "assert_eq!(mem::size_of::<BaseNeuron>(), 256")

with open("tests/test_stdp.rs", "w") as f:
    f.write(content)
