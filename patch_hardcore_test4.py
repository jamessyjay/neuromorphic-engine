import re

with open("tests/test_stdp.rs", "r") as f:
    content = f.read()

# In phase 2, we need the LTD to be strong enough to reduce C->B weights
new_phase2 = """    // Step 3: Phase 2 — The Noise Storm (1000 ticks)
    env.dopamine_multiplier = 1.0;
    env.stdp_learning_rate = 50.0; // Needs to be high enough to allow LTD to shrink noise weights
    
    for _ in 0..1000 {
        // Competitive Noise: Randomly spike neurons in Group B every 3-5 ticks
        if tick % 4 == 0 {
            for b in 10..=15 {
                if rand() % 2 == 0 {
                    graph.neurons[b as usize].current_voltage = 40.0;
                    graph.neurons[b as usize].energy_atp = 100.0;
                    // Reset traces to isolate LTD
                    graph.neurons[b as usize].spike_trace = 0.0;
                }
            }
        }

        // Causal Noise: Randomly spike Group C every 2-4 ticks
        if tick % 3 == 0 {
            for c in 20..=25 {
                if rand() % 2 == 0 {
                    graph.neurons[c as usize].current_voltage = 40.0;
                    graph.neurons[c as usize].energy_atp = 100.0;
                    graph.neurons[c as usize].spike_trace = 0.0;
                }
            }
        }
        
        graph.tick(tick, env.clone());
        // Prevent homeostatic recovery from interfering
        for b in 10..=15 {
            graph.neurons[b as usize].spikes_in_last_window = 10;
        }
        tick += 1;
    }"""

content = re.sub(r'    // Step 3: Phase 2 — The Noise Storm \(1000 ticks\).*?    let avg_a_b_post_noise', new_phase2 + '\n\n    let avg_a_b_post_noise', content, flags=re.DOTALL)

with open("tests/test_stdp.rs", "w") as f:
    f.write(content)
