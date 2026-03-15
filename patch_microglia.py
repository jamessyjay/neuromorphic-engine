import re

with open("src/network/graph.rs", "r") as f:
    content = f.read()

content = content.replace("use rand::Rng;\n", "")

with open("src/network/graph.rs", "w") as f:
    f.write(content)
