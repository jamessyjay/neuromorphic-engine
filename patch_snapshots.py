import re

with open("src/runner/modes.rs", "r") as f:
    content = f.read()

pattern = r'BrainSnapshot \{\s*current_tick:\s*(.*?),\s*environment:\s*Environment\s*\{[\s\S]*?wakefulness_multiplier:\s*1\.0\s*\},?\s*graph:\s*graph\.clone\(\)\s*\}\.save_to_disk\((.*?)\);'
replacement = r'BrainSnapshot::save_test_snapshot(\1, &graph, \2);'

new_content = re.sub(pattern, replacement, content)

with open("src/runner/modes.rs", "w") as f:
    f.write(new_content)
