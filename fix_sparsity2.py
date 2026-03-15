import re

# Okay, `add_neuron` does sequentially assign IDs.
# So `self.neurons[id as usize]` IS perfectly correct.
# Wait! In `apply_wta_inhibition` I had this code:
# `for n in &self.neurons { if n.current_voltage > n.activation_threshold ... }`
# BUT `graph.neurons[i].current_voltage = 50.0 + (i % 20) as f32;`
# So `current_voltage` IS > `activation_threshold`.
# BUT why did 1000 neurons spike?
# BECAUSE `graph.tick(1, env)` DOES THIS:
# 1. `apply_wta_inhibition` -> Sets 900 losers to `v = -80.0`.
# 2. THEN `tick_neuron_cpu` runs!
# WAIT! `tick_neuron_cpu` receives `&mut neuron` using Rayon:
# `self.neurons.par_iter_mut().for_each(|n| { unsafe { tick_neuron_cpu(n, env, current_tick); } });`
# Inside `tick_neuron_cpu`, what does `process_spike` do?
# `if (n->current_voltage > dynamic_threshold) { ... }`
# DOES IT READ `n->current_voltage` AS -80.0?
# IF `apply_wta_inhibition` set it to -80.0, WHY WOULD IT READ ANYTHING ELSE?
# UNLESS `apply_wta_inhibition` DID NOT SET IT TO -80.0!

# Let's print out the ACTUAL voltage right before `process_spike` in C++!
# Or we can just print the whole `minicolumn_winners` hash map in Rust to see if it even found winners.

with open("src/network/graph.rs", "r") as f:
    graph_content = f.read()

# Let's add a temporary panic to see the winners map size
graph_content = graph_content.replace(
    "// if self.neurons.len() >= 1000 { println!(\"WTA Winners count: {}\", minicolumn_winners.len()); }",
    "if self.neurons.len() >= 1000 { println!(\"WTA Winners count: {}\", minicolumn_winners.len()); }"
)

with open("src/network/graph.rs", "w") as f:
    f.write(graph_content)

