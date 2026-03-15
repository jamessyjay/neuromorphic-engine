import re

with open("tests/sparsity_tests.rs", "r") as f:
    content = f.read()

# Why did sparsity test fail?
# We have 1000 neurons, we force_sync_gpu() (which sets activation threshold to ~ -52.0).
# Then we blast them with `current_voltage = 50.0 + i % 20`.
# Then `graph.apply_wta_inhibition()` runs (via `graph.tick(1, env)`).
# The WTA logic checks `if n.current_voltage > n.activation_threshold`.
# Since 50.0 > -52.0, ALL 1000 neurons are considered.
# For each minicolumn (10 neurons), one winner is selected (the one with highest voltage).
# The other 9 get `n.status_flags |= FLAG_IS_INHIBITED;` and `n.current_voltage = n.resting_voltage - 10.0;`.
# The resting voltage is ~ -70.0. So voltage becomes ~ -80.0.
# Then `tick_neuron_cpu` runs.
# Inside `tick_neuron_cpu`, `process_spike` is called.
# It checks `if (n->current_voltage > dynamic_threshold)`.
# For the 900 losers, current_voltage is -80.0, which is NOT > -52.0.
# So they do NOT spike!
# So WHY did `graph.neurons.iter().filter(|n| n.last_spike_tick == 1).count()` return 1000?!

# Oh! Wait! Look at `fix_columnar_tests3.py` which I used to change `tests/sparsity_tests.rs`:
# `graph.neurons[i].current_voltage = 50.0 + (i % 20) as f32;`
# But in `graph.rs`:
# `let current_winner_v = minicolumn_winners.get(&key).map(|&id| self.neurons[id as usize].current_voltage).unwrap_or(-1000.0);`
# `if n.current_voltage > current_winner_v { minicolumn_winners.insert(key, n.id); }`
# Is `id as usize` CORRECT?!
# If `n.id` is 0..1000, then `id as usize` is correct.
# Let's check `tests/sparsity_tests.rs`.
# Is `id` actually sequential from 0 to 999?
# Yes, `for i in 0..1000 { let id = graph.add_neuron(...); }`
# Wait... Does `add_neuron` return the ID? Let's check `graph.rs`!
