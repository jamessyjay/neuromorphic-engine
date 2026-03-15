import re

with open("tests/biology_tests.rs", "r") as f:
    content = f.read()

# In `test_metabolic_sink`:
# We force `n.current_voltage = 100.0` for ticks 5001..=5200.
# BUT wait! We added `apply_wta_inhibition` to `graph.tick()`!
# Since all neurons have `current_voltage = 100.0` and they all have `column_id = 0, minicolumn_id = 0`,
# WTA will pick ONE winner per minicolumn and inhibit ALL THE REST!
# The 100 neurons are in the SAME minicolumn (because `column_id` and `minicolumn_id` are 0).
# So ONLY 1 neuron will spike!
# The other 99 neurons will be inhibited to `current_voltage = -80.0`.
# Since they don't spike, they don't consume `ATP_SPIKE_COST`!
# They only consume `passive_drain`.
# In 200 ticks, `passive_drain` is not enough to kill them if they don't spike!
# This is why 96 neurons are still alive!

# Fix: We need to assign each neuron to a DIFFERENT minicolumn so they don't inhibit each other.
content = content.replace("graph.add_neuron(NeuronType::Pyramidal);", "graph.add_neuron(NeuronType::Pyramidal);\n        let id = graph.neurons.len() - 1;\n        graph.neurons[id].column_id = id as u16;\n        graph.neurons[id].minicolumn_id = 1;")

with open("tests/biology_tests.rs", "w") as f:
    f.write(content)

