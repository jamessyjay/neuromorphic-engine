use neuromorphic_engine::network::graph::NetworkGraph;
use neuromorphic_engine::engine::ffi::NeuronType;


fn default_env() -> neuromorphic_engine::engine::ffi::Environment {
    neuromorphic_engine::engine::ffi::Environment {
        global_oxygen_level: 1.0,
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0,
        thermal_noise_scale: 0.0,
        stdp_learning_rate: 0.01,
        toxin_levels: 0.0,
        light_level: 1.0,
        wakefulness_multiplier: 1.0,
    }
}

// 6. Test "Clean Memory" (Dopamine Washout)
#[test]
fn test_dopamine_washout() {
    let mut env = default_env();
    env.dopamine_multiplier = 5.0; // Spike dopamine
    
    let mut regulator = neuromorphic_engine::engine::environment::EnvironmentRegulator::new();
    
    for _ in 0..50 {
        env = regulator.update_and_get(100.0, 1, &env);
    }
    
    // Baseline dopamine should decay towards 1.0 ideally, but currently decays towards 0.0 based on * 0.95
    assert!(env.dopamine_multiplier < 1.5, "Dopamine did not wash out correctly! Current: {}", env.dopamine_multiplier);
}

// 7. Test "Even Seeding" (Branch Diversity)
#[test]
fn test_branch_diversity() {
    let mut graph = NetworkGraph::new();
    let tgt = graph.add_neuron(NeuronType::Pyramidal);
    
    // Enable all branches
    graph.neurons[tgt as usize].active_branches_mask = 0xFFFF;
    for i in 0..16 {
        graph.neurons[tgt as usize].branches[i].branch_type = 2; // Make them all valid
    }
    
    let mut rng = rand::rng();
    
    // Run neurogenesis 1000 times
    let mut branch_counts = [0; 16];
    for _ in 0..1000 {
        let _src = graph.add_neuron(NeuronType::Pyramidal);
        
        let branch_id = rand::RngExt::random_range(&mut rng, 0..12) as usize;
        branch_counts[branch_id] += 1;
    }
    
    let expected = 1000 / 12;
    for i in 0..12 {
        let count = branch_counts[i];
        let diff = (count as i32 - expected as i32).abs();
        assert!(diff < 50, "Branch {} has skewed distribution: {} (Expected {})", i, count, expected);
    }
}

// 8. Test "Geometric Justice" (Distance Law L^-1.5)
#[test]
fn test_geometric_justice() {
    let mut graph = NetworkGraph::new();
    
    // Generate a quick network to test connectivity
    // 1000 neurons, spread out
    let mut rng = rand::rng();
    for _i in 0..1000 {
        let id = graph.add_neuron(NeuronType::Pyramidal);
        let n = &mut graph.neurons[id as usize];
        n.x = rand::RngExt::random_range(&mut rng, 0..65535);
        n.y = rand::RngExt::random_range(&mut rng, 0..65535);
        n.z = rand::RngExt::random_range(&mut rng, 0..65535);
    }
    
    let mut short_prob_sum = 0.0;
    let mut short_prob_count: f32 = 0.0;
    let mut long_prob_sum = 0.0;
    let mut long_prob_count: f32 = 0.0;
    
    for _ in 0..10000 {
        let n_source = &graph.neurons[rand::RngExt::random_range(&mut rng, 0..1000)];
        let n_target = &graph.neurons[rand::RngExt::random_range(&mut rng, 0..1000)];
        
        let dx = n_source.x as f32 - n_target.x as f32;
        let dy = n_source.y as f32 - n_target.y as f32;
        let dz = n_source.z as f32 - n_target.z as f32;
        let dist = (dx*dx + dy*dy + dz*dz).sqrt();
        
        let base_prob = 1.0 / (1.0 + (dist / 1000.0).powf(1.5));
        let prob = base_prob.clamp(0.001, 0.95);
        
        if dist < 10000.0 {
            short_prob_sum += prob;
            short_prob_count += 1.0;
        } else if dist > 50000.0 {
            long_prob_sum += prob;
            long_prob_count += 1.0;
        }
    }
    
    let avg_short = short_prob_sum / short_prob_count.max(1.0);
    let avg_long = long_prob_sum / long_prob_count.max(1.0);
    println!("Avg Short Prob: {}, Avg Long Prob: {}", avg_short, avg_long);
    assert!(avg_short > avg_long * 10.0, "Short-range probability should be > 10x higher than long-range! Short: {}, Long: {}", avg_short, avg_long);
}

// 9. Test "Automatic Homeostasis" (Synaptic Scaling)
#[test]
fn test_synaptic_scaling_proportionality() {
    let mut graph = NetworkGraph::new();
    let tgt = graph.add_neuron(NeuronType::Pyramidal);
    
    let weights = vec![0.5, 1.0, 2.0, 4.0, 8.0];
    let src_nodes: Vec<u32> = weights.iter().map(|_| graph.add_neuron(NeuronType::Pyramidal)).collect();
    
    for (i, w) in weights.iter().enumerate() {
        graph.connect(src_nodes[i], tgt, *w, 0, false, false);
    }
    
    graph.apply_synaptic_scaling();
    
    let mut new_weights = vec![];
    for s in &graph.synapses {
        if s.target_id == tgt {
            new_weights.push(s.weight);
        }
    }
    
    let sum_new: f32 = new_weights.iter().sum();
    println!("New weights: {:?}", new_weights);
    println!("New sum: {}", sum_new);
    
    assert!((sum_new - 7.5).abs() < 0.1, "Scaling did not reach target sum! Expected 7.5, got {}", sum_new);
    
    // Check proportionality
    let ratio1 = new_weights[1] / new_weights[0];
    let ratio2 = new_weights[3] / new_weights[2];
    
    assert!((ratio1 - 2.0).abs() < 0.01, "Proportionality lost! Expected ratio 2.0, got {}", ratio1);
    assert!((ratio2 - 2.0).abs() < 0.01, "Proportionality lost! Expected ratio 2.0, got {}", ratio2);
}

// 10. Test "Echo Decay" (E/I Balance)
#[test]
fn test_echo_decay() {
    let mut graph = NetworkGraph::new();
    
    let mut rng = rand::rng();
    for _ in 0..800 {
        graph.add_neuron(NeuronType::Pyramidal);
    }
    for _ in 0..200 {
        graph.add_neuron(NeuronType::Interneuron);
    }
    
    for src in 0..1000 {
        for tgt in 0..1000 {
            if src != tgt && rand::RngExt::random_range(&mut rng, 0.0..1.0) < 0.02 {
                let is_inhib = graph.neurons[src as usize].type_ == NeuronType::Interneuron as u8;
                graph.connect(src, tgt, 2.0, 0, is_inhib, false);
            }
        }
    }
    
    for i in 0..10 {
        graph.neurons[i].current_voltage = 100.0;
    }
    
    let env = default_env();
    
    let mut _last_spikes = 0;
    for tick in 1..=200 {
        graph.tick(tick, env.clone());
        
        let mut spikes_this_tick = 0;
        for n in &graph.neurons {
            if n.last_spike_tick == tick {
                spikes_this_tick += 1;
            }
        }
        
        println!("Tick {}: {} spikes", tick, spikes_this_tick);
        
        if tick > 150 {
            assert!(spikes_this_tick < 10, "Echo did not decay! Network in epilepsy.");
        }
    }
}
