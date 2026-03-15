import re

with open("tests/dendrite_tests.rs", "r") as f:
    content = f.read()

lines = content.splitlines()
start = -1
for i, line in enumerate(lines):
    if "fn test_dendritic_role_separation" in line:
        start = i
        break

end = start
open_braces = 0
for i in range(start, len(lines)):
    open_braces += lines[i].count('{')
    open_braces -= lines[i].count('}')
    if open_braces == 0:
        end = i
        break

print("\n".join(lines[start:end+1]))

