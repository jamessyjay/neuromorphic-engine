# The issue is that `predictive_ticks` is 0 instead of 20.
# Why didn't the distal branch trigger an NMDA spike?
# Because `voltage > nmda_threshold` wasn't met!
# Let's check `process_synapse` in `cuda/base_neuron.cu`.
# Synapses with `flags` representing branch id deliver neurotransmitter to `target->branches[branch_id].voltage`.
# The test sets `flags: 6` (Branch 6). 
# But wait! Does the synapse weight 5.0 push the voltage above `nmda_threshold`?
# What is the default `nmda_threshold` of a Compartment?
# Wait! In `init_neuron` in C++ (`cuda/base_neuron.cu`), how are `branches` initialized?
import re

with open("cuda/base_neuron.cu", "r") as f:
    cuda_content = f.read()

lines = cuda_content.splitlines()
start = -1
for i, line in enumerate(lines):
    if "void init_neuron" in line:
        start = i
        break

if start != -1:
    end = start
    open_braces = 0
    for i in range(start, len(lines)):
        open_braces += lines[i].count('{')
        open_braces -= lines[i].count('}')
        if open_braces == 0:
            end = i
            break
    print("\n".join(lines[start:end+1]))

