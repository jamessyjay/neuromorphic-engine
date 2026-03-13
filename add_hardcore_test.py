import re

with open("tests/test_stdp.rs", "r") as f:
    content = f.read()

new_test = """
#[test]
fn test_stdp_associative_persistence_under_noise() {
    let mut graph = NetworkGraph::new();
    for _ in 0..100 {
        let n = graph.add_neuron(NeuronType::Pyramidal);
        // Explicitly enable dendritic branch 0 (Basal) so signals can reach the soma
        graph.neurons[n as usize].branches[0].branch_type = 1;
    }

    let mut env = default_env();

    let mut a_to_b = vec![];
    for a in 0..=5 {
        for b in 10..=15 {
            let syn = Synapse {
                source_id: a, target_id: b, weight: 1.0,
                neurotransmitter_buffer: 0.0, last_activation_tick: 0,
                ticks_below_threshold: 0, ticks_until_delivery: 0, flags: 0, _padding: [0; 2]
            };
            a_to_b.push(graph.synapses.len());
            graph.synapses.push(syn);
        }
    }

    let mut c_to_b = vec![];
    for c in 20..=25 {
        for b in 10..=15 {
            let syn = Synapse {
                source_id: c, target_id: b, weight: 1.0,
                neurotransmitter_buffer: 0.0, last_activation_tick: 0,
                ticks_below_threshold: 0, ticks_until_delivery: 0, flags: 0, _padding: [0; 2]
            };
            c_to_b.push(graph.synapses.len());
            graph.synapses.push(syn);
        }
    }

    // Add random noise synapses up to ~1000 total
    let mut seed: u64 = 12345;
    let mut rand = || {
        seed = (seed * 1103515245 + 12345) % 2147483648;
        seed as u32
    };

    while graph.synapses.len() < 1000 {
        let src = rand() % 100;
        let tgt = rand() % 100;
        if src != tgt {
            let syn = Synapse {
                source_id: src, target_id: tgt, weight: 1.0,
                neurotransmitter_buffer: 0.0, last_activation_tick: 0,
                ticks_below_threshold: 0, ticks_until_delivery: 0, flags: 0, _padding: [0; 2]
            };
            graph.synapses.push(syn);
        }
    }

    // PHASE 1: Associative Conditioning
    env.stdp_learning_rate = 50.0; // Needs to be high to reach 3.0 in 100 cycles 
    env.dopamine_multiplier = 5.0;
    
    let mut tick = 1;
    for _cycle in 0..100 {
        // Prevent homeostatic scaling from messing things up during training
        for n in &mut graph.neurons {
            n.spikes_in_last_window = 10;
        }

        // Tick T: Spike Group A
        for a in 0..=5 {
            graph.neurons[a as usize].current_voltage = 40.0;
            graph.neurons[a as usize].energy_atp = 100.0;
        }
        graph.tick(tick, env.clone());
        tick += 1;

        // Wait to Tick T+20
        for _ in 0..19 {
            graph.tick(tick, env.clone());
            tick += 1;
        }

        // Tick T+20: Spike Group B
        for b in 10..=15 {
            graph.neurons[b as usize].current_voltage = 40.0;
            graph.neurons[b as usize].energy_atp = 100.0;
        }
        graph.tick(tick, env.clone());
        tick += 1;

        // Rest for a bit to avoid continuous spiking
        for _ in 0..29 {
            graph.tick(tick, env.clone());
            tick += 1;
        }
    }

    let avg_a_b_post_train: f32 = a_to_b.iter().map(|&i| graph.synapses[i].weight).sum::<f32>() / a_to_b.len() as f32;
    println!("Average A->B Weight post-training: {:.4}", avg_a_b_post_train);

    // PHASE 2: The Noise Storm
    env.dopamine_multiplier = 1.0;
    env.stdp_learning_rate = 5.0; // Just in case, to make sure noise LTD can reduce it if it's uncorrelated
    
    for _ in 0..1000 {
        for n in &mut graph.neurons {
            n.spikes_in_last_window = 10;
        }

        if tick % 3 == 0 || tick % 5 == 0 {
            for b in 10..=15 {
                if rand() % 2 == 0 {
                    graph.neurons[b as usize].current_voltage = 40.0;
                    graph.neurons[b as usize].energy_atp = 100.0;
                    // Reset traces to isolate LTD
                    graph.neurons[b as usize].spike_trace = 0.0;
                }
            }
        }

        if tick % 2 == 0 || tick % 4 == 0 {
            for c in 20..=25 {
                if rand() % 2 == 0 {
                    graph.neurons[c as usize].current_voltage = 40.0;
                    graph.neurons[c as usize].energy_atp = 100.0;
                    graph.neurons[c as usize].spike_trace = 0.0;
                }
            }
        }
        
        graph.tick(tick, env.clone());
        tick += 1;
    }

    let avg_a_b_post_noise: f32 = a_to_b.iter().map(|&i| graph.synapses[i].weight).sum::<f32>() / a_to_b.len() as f32;
    println!("Average A->B Weight post-noise: {:.4}", avg_a_b_post_noise);

    // PHASE 3: Pavlovian Validation
    for _ in 0..50 {
        graph.tick(tick, env.clone());
        tick += 1;
    }

    let final_a_b: f32 = a_to_b.iter().map(|&i| graph.synapses[i].weight).sum::<f32>() / a_to_b.len() as f32;
    let final_c_b: f32 = c_to_b.iter().map(|&i| graph.synapses[i].weight).sum::<f32>() / c_to_b.len() as f32;
    
    println!("Final weight of a random A->B synapse: {:.4}", graph.synapses[a_to_b[0]].weight);
    println!("Final weight of a random C->B synapse: {:.4}", graph.synapses[c_to_b[0]].weight);

    assert!(final_a_b > 3.0 && final_a_b <= 5.0, "Average A->B weight must be between 3.0 and 5.0, was {}", final_a_b);
    assert!(final_c_b < 1.5, "Average C->B weight (noise) must be below 1.5, was {}", final_c_b);

    // Stimulus: Spike Group A exactly once
    for a in 0..=5 {
        graph.neurons[a as usize].current_voltage = 40.0;
        graph.neurons[a as usize].energy_atp = 100.0;
    }
    graph.tick(tick, env.clone());
    tick += 1;

    let mut b_spiked = false;
    for _ in 0..20 {
        graph.tick(tick, env.clone());
        for b in 10..=15 {
            if graph.neurons[b as usize].last_spike_tick == tick {
                b_spiked = true;
            }
        }
        tick += 1;
    }
    assert!(b_spiked, "Group B failed to spike from Pavlovian response!");

    // PHASE 4: Biological Integrity
    for _ in 0..20 {
        graph.tick(tick, env.clone());
        tick += 1;
    }

    let mut quiet_spikes = 0;
    for _ in 0..50 {
        graph.tick(tick, env.clone());
        for n in &graph.neurons {
            if n.last_spike_tick == tick {
                quiet_spikes += 1;
            }
        }
        tick += 1;
    }
    assert!(quiet_spikes < 10, "Network should return to a quiet state, but had {} spikes (Epilepsy check failed)", quiet_spikes);

    for b in 10..=15 {
        assert!(graph.neurons[b as usize].energy_atp > 0.0, "Target neuron starved!");
    }

    for a in 0..=5 {
        let trace = graph.neurons[a as usize].spike_trace;
        assert!(trace < 0.1, "Spike trace for group A did not decay properly, was {}", trace);
    }
}
"""

content = content.replace("#[test]\nfn bench_stdp_overhead() {", new_test + "\n#[test]\nfn bench_stdp_overhead() {")

with open("tests/test_stdp.rs", "w") as f:
    f.write(content)
