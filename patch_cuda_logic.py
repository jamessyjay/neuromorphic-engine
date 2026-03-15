import re

with open("cuda/base_neuron.cu", "r") as f:
    content = f.read()

# Clear is_inhibited at the beginning of the tick, and handle 16 branches
# Let's find tick_neurons_kernel to add the flag clear
# We also need to see how branches are looped

# Wait, `evaluate_branches` might have hardcoded 8

def replace_branches(match):
    return match.group(0).replace("NUM_BRANCHES", "16").replace("i < 8", "i < 16")

# In cuda, NUM_BRANCHES might be hardcoded as 8 in evaluate_branches
content = re.sub(r'#define NUM_BRANCHES 8', '#define NUM_BRANCHES 16', content)

# Check for hardcoded 8 in evaluate_branches loop
content = re.sub(r'for\s*\(\s*int\s*i\s*=\s*0;\s*i\s*<\s*8;\s*\+\+i\s*\)', 'for (int i = 0; i < 16; ++i)', content)

# Clear FLAG_IS_INHIBITED in tick_neurons_kernel
# Wait, we need to know what tick_neurons_kernel looks like
with open("cuda/base_neuron.cu", "w") as f:
    f.write(content)
