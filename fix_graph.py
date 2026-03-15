import re

with open('src/network/graph.rs', 'r') as f:
    code = f.read()

code = re.sub(
    r'self\.neurons\.par_iter_mut\(\)\.for_each\(\|n\| \{\n\s*// To avoid changing tick_neuron_cpu signature again, pass null for all_neurons\n\s*unsafe \{ crate::engine::ffi::tick_neuron_cpu\(n, std::ptr::null_mut\(\), 0, env, current_tick\); \}\n\s*\}\);',
    '''self.neurons.par_iter_mut().for_each(|n| {
                unsafe { crate::engine::ffi::tick_neuron_prepare_cpu(n, env, current_tick); }
            });''',
    code
)

code = re.sub(
    r'            // Phase 2: Broadcast \(WTA\)\n            use std::collections::HashMap;\n            let mut minicolumn_winners: HashMap<\(u16, u8\), u32> = HashMap::new\(\);\n            for n in &self\.neurons \{\n                if n\.is_alive == 1 && n\.last_spike_tick == current_tick && \(n\.type_ == 0 \|\| n\.type_ == 2\) \{\n                    let key = \(n\.column_id, n\.minicolumn_id\);\n                    let current_winner = minicolumn_winners\.entry\(key\)\.or_insert\(n\.id\);\n                    if n\.id < \*current_winner \{\n                        \*current_winner = n\.id;\n                    \}\n                \}\n            \}\n\n            for n in &mut self\.neurons \{\n                if n\.is_alive == 1 \{\n                    let key = \(n\.column_id, n\.minicolumn_id\);\n                    if let Some\(&winner_id\) = minicolumn_winners\.get\(&key\) \{\n                        if n\.id != winner_id \{\n                            n\.status_flags \|= crate::engine::constants::FLAG_IS_INHIBITED;\n                            n\.current_voltage = n\.resting_voltage;\n                            n\.refractory_time_left = 5\.0;\n                            if n\.last_spike_tick == current_tick \{\n                                // Rollback spike for suppressed losers\n                                n\.last_spike_tick = current_tick\.saturating_sub\(1\);\n                                n\.spikes_in_last_window = n\.spikes_in_last_window\.saturating_sub\(1\);\n                            \}\n                        \}\n                    \}\n                \}\n            \}',
    '''            // Phase 2: WTA and Bursting
            use std::collections::{HashMap, HashSet};
            let mut minicolumn_winners: HashMap<(u16, u8), u32> = HashMap::new();
            let mut predictive_crossed: HashSet<(u16, u8)> = HashSet::new();
            
            for n in &self.neurons {
                if n.is_alive == 1 && n.current_voltage > n.activation_threshold && (n.type_ == 0 || n.type_ == 2) {
                    let key = (n.column_id, n.minicolumn_id);
                    let is_pred = (n.status_flags & crate::engine::constants::FLAG_IS_PREDICTIVE) != 0;
                    
                    if is_pred {
                        predictive_crossed.insert(key);
                        let current_winner = minicolumn_winners.entry(key).or_insert(n.id);
                        if n.id < *current_winner {
                            *current_winner = n.id;
                        }
                    } else if !predictive_crossed.contains(&key) {
                        let current_winner = minicolumn_winners.entry(key).or_insert(n.id);
                        if n.id < *current_winner {
                            *current_winner = n.id;
                        }
                    }
                }
            }

            for n in &mut self.neurons {
                if n.is_alive == 1 && (n.type_ == 0 || n.type_ == 2) {
                    let key = (n.column_id, n.minicolumn_id);
                    if let Some(&winner_id) = minicolumn_winners.get(&key) {
                        if predictive_crossed.contains(&key) {
                            if n.id != winner_id {
                                n.status_flags |= crate::engine::constants::FLAG_IS_INHIBITED;
                                n.current_voltage = n.resting_voltage - 10.0;
                                n.refractory_time_left = 5.0;
                            }
                        } else {
                            // Bursting
                            if n.current_voltage <= n.activation_threshold {
                                n.current_voltage = n.activation_threshold + 5.0;
                            }
                        }
                    }
                }
            }
            
            self.neurons.par_iter_mut().for_each(|n| {
                unsafe { crate::engine::ffi::tick_neuron_spike_cpu(n, env, current_tick); }
            });''',
    code
)

with open('src/network/graph.rs', 'w') as f:
    f.write(code)

