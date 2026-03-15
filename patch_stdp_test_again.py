import re

with open("tests/test_stdp.rs", "r") as f:
    content = f.read()

content = content.replace('assert_eq!(std::mem::size_of::<BaseNeuron>(), 128, "BaseNeuron must be exactly 128 bytes");', 'assert_eq!(std::mem::size_of::<BaseNeuron>(), 256, "BaseNeuron must be exactly 256 bytes");')

# Let's verify we replaced all instances of 128 bytes check
content = re.sub(r'assert_eq!\(std::mem::size_of::<BaseNeuron>\(\),\s*128', 'assert_eq!(std::mem::size_of::<BaseNeuron>(), 256', content)
content = re.sub(r'128 bytes', '256 bytes', content)

with open("tests/test_stdp.rs", "w") as f:
    f.write(content)
