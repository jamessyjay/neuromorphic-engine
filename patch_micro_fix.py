import re

with open("src/network/graph.rs", "r") as f:
    content = f.read()

# Let's check how many synapses get deleted in neurogenesis vs pruning
# Wait, let's output some debug information in run_microglia_service
replacement = """    // Background Service: Microglia (Immune System / Firewall / Pruning)
    pub fn run_microglia_service(&mut self, current_tick: u32) {
        let neurons_ptr = self.neurons.as_ptr();
        
        let num_neurons = self.neurons.len();
        let mut rng = rand::rng();

        // O(1) Pre-compute outgoing synapse counts to support competitive pruning
        let mut outgoing_counts = vec![0; num_neurons];
        for s in &self.synapses {
            if (s.source_id as usize) < num_neurons {
                outgoing_counts[s.source_id as usize] += 1;
            }
        }

        let mut pruned = 0;
        self.synapses.retain(|s| {
            // Safety Guard: Prevent out-of-bounds segfaults if the graph is modified
            if s.source_id as usize >= num_neurons || s.target_id as usize >= num_neurons {
                return false;
            }

            let src_id = s.source_id as usize;

            // Retain only synapses where both endpoints are alive.
            let (src_valid, dst_valid) = unsafe {
                let src = &*neurons_ptr.add(src_id);
                let dst = &*neurons_ptr.add(s.target_id as usize);
                (src.is_alive == 1 || src.dead_ticks > 0, dst.is_alive == 1 || dst.dead_ticks > 0)
            };
            
            if !src_valid || !dst_valid {
                return false;
            }

            // Competitive Pruning: Ensure neuron maintains MIN_SYNAPSES_PER_NEURON
            if outgoing_counts[src_id] <= crate::engine::constants::MIN_SYNAPSES_PER_NEURON {
                return true;
            }

            // Probabilistic Pruning Logic
            let time_since_spike = current_tick.saturating_sub(s.last_activation_tick);
            
            let p_drop = if s.weight < 0.1 && time_since_spike > 10000 {
                1.0
            } else if s.weight < crate::engine::constants::PRUNING_THRESHOLD {
                0.05
            } else {
                0.0
            };

            if p_drop > 0.0 && rng.random_bool(p_drop) {
                outgoing_counts[src_id] -= 1;
                pruned += 1;
                return false; // Prune
            }

            true
        });
        
        // Uncomment to debug pruning aggressiveness
        // if pruned > 0 { println!("Microglia pruned {} synapses at tick {}", pruned, current_tick); }
    }"""

content = re.sub(r'    // Background Service: Microglia \(Immune System / Firewall / Pruning\)[\s\S]*?        \}\);\n        \n        // Uncomment to debug pruning aggressiveness\n        // if pruned > 0 \{ println!\("Microglia pruned \{\} synapses at tick \{\}", pruned, current_tick\); \}\n    \}', replacement, content)

with open("src/network/graph.rs", "w") as f:
    f.write(content)
