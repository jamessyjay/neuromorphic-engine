use neuromorphic_engine::network::graph::NetworkGraph;
use neuromorphic_engine::engine::ffi::{NeuronType, Environment};
use neuromorphic_engine::engine::constants;

fn default_env() -> Environment {
    Environment {
        global_oxygen_level: 1.0,
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0,
        toxin_levels: 0.0,
        light_level: 0.0,
        stdp_learning_rate: 0.1,
        wakefulness_multiplier: 1.0,
            thermal_noise_scale: 1.0,
    }
}

fn pack_voltage(v: f32) -> u16 {
    ((v + 100.0) * 100.0).clamp(0.0, 65535.0) as u16
}

#[test]
fn test_sequence_learning() {
    let mut graph = NetworkGraph::new();
    
    // Pattern 1 (Pixel X) -> Neuron 0
    // Pattern 2 (Pixel Y) -> Minicolumn 1 (Neurons 1 to 20)
    
    graph.add_neuron(NeuronType::Pyramidal); // 0 (Pattern 1)
    graph.neurons[0].column_id = 0;
    graph.neurons[0].minicolumn_id = 0;
    
    for _ in 1..=20 {
        let n = graph.add_neuron(NeuronType::Pyramidal);
        graph.neurons[n as usize].column_id = 1;
        graph.neurons[n as usize].minicolumn_id = 1;
        graph.neurons[n as usize].branches[4].branch_type = 1; // Basal (Distal)
        graph.neurons[n as usize].branches[4].nmda_threshold = pack_voltage(10.0);
    }
    
    // Connect Pattern 1 to all neurons in Pattern 2's minicolumn (Distal connections)
    for i in 1..=20 {
        graph.connect(0, i, 5.0, 4, false, false); 
    }
    
    let mut env = default_env();
    env.dopamine_multiplier = 5.0; // High learning rate
    
    // Train 10 times
    for cycle in 0..10 {
        let base_tick = cycle * 10;
        
        // Pattern 1 fires
        graph.neurons[0].current_voltage = 50.0;
        graph.tick(base_tick + 1, env.clone());
        
        // Pattern 2 fires shortly after (WTA allows one to win if it's already slightly ahead, or they burst. Let's just burst them or force spike)
        for i in 1..=20 {
            graph.neurons[i as usize].current_voltage = 50.0;
        }
        graph.tick(base_tick + 2, env.clone());
        graph.tick(base_tick + 3, env.clone());
    }
    
    // Reset voltages and states
    for n in &mut graph.neurons {
        n.current_voltage = n.resting_voltage;
        n.status_flags = 0;
        n.predictive_ticks = 0;
        n.branches[4].voltage = pack_voltage(0.0);
    }
    
    // 11th time: only Pattern 1 fires
    graph.neurons[0].current_voltage = 50.0;
    graph.tick(200, env.clone());
    graph.tick(201, env.clone()); // Delivery
    graph.tick(202, env.clone()); // Summation
    graph.tick(203, env.clone()); // Evaluation & Predictive set
    graph.tick(204, env.clone());
    
    // Assert Pattern 2 neurons enter Predictive State
    let mut predictive_count = 0;
    for i in 1..=20 {
        if (graph.neurons[i as usize].status_flags & constants::FLAG_IS_PREDICTIVE) != 0 {
            predictive_count += 1;
        }
    }
    
    let test_v = ((graph.neurons[1].branches[4].voltage as f32) / 100.0) - 100.0; println!("Predictive count: {}, Branch V: {}, Status: {}", predictive_count, test_v, graph.neurons[1].status_flags);
    assert!(predictive_count > 0, "At least some neurons in Pattern 2 should be in FLAG_IS_PREDICTIVE state");
}

#[test]
fn test_wta_bulletin_board_o1() {
    let mut graph = NetworkGraph::new();
    
    // Create 5 pyramidal neurons in the same minicolumn
    for _ in 0..5 {
        let n = graph.add_neuron(NeuronType::Pyramidal);
        graph.neurons[n as usize].column_id = 42;
        graph.neurons[n as usize].minicolumn_id = 7;
        graph.neurons[n as usize].current_voltage = -60.0; // below threshold
    }
    
    // Artificially raise voltage of neuron 3 to win
    graph.neurons[3].current_voltage = 50.0; // Clearly above threshold
    graph.neurons[3].status_flags |= constants::FLAG_IS_PREDICTIVE; // Must be predictive to inhibit others!
    graph.neurons[1].current_voltage = 10.0; // Also above threshold, but lower than 3
    
    let env = default_env();
    graph.tick(1, env);
    
    #[cfg(feature = "cuda")]
    {
        // For CUDA, we can verify via FFI directly
        let mut out_v = 0.0f32;
        let mut out_id = 0u32;
        unsafe {
            neuromorphic_engine::engine::ffi::debug_get_minicolumn_winner(42, 7, &mut out_v, &mut out_id);
        }
        
        assert_eq!(out_id, 4, "Neuron with index 3 should have won the WTA bulletin board (its ID is 4)");
        assert!(out_v > 1049.0 && out_v < 1055.0, "Winner voltage should be around 1050 (50 + 1000 for predictive)");
        
        // Verify others are inhibited
        assert_eq!((graph.neurons[1].status_flags & constants::FLAG_IS_INHIBITED) != 0, true, "Neuron 1 should be inhibited");
        assert_eq!((graph.neurons[3].status_flags & constants::FLAG_IS_INHIBITED) == 0, true, "Neuron 3 (winner) should NOT be inhibited");
    }
    
    #[cfg(not(feature = "cuda"))]
    {
        // In CPU mode, the WTA broadcast isn't fully using the bulletin board in the same way, but the result should be the same.
        assert_eq!((graph.neurons[1].status_flags & constants::FLAG_IS_INHIBITED) != 0, true, "Neuron 1 should be inhibited");
        assert_eq!((graph.neurons[3].status_flags & constants::FLAG_IS_INHIBITED) == 0, true, "Neuron 3 (winner) should NOT be inhibited");
    }
}

#[test]
fn test_column_bursting() {
    let mut graph = NetworkGraph::new();
    
    for _ in 0..20 {
        let n = graph.add_neuron(NeuronType::Pyramidal);
        graph.neurons[n as usize].column_id = 1;
        graph.neurons[n as usize].minicolumn_id = 1;
        graph.neurons[n as usize].layer_id = 2;
    }
    
    let env = default_env();
    
    // Provide a totally new stimulus (feedforward to all neurons in minicolumn)
    // No neuron is in predictive state.
    for i in 0..20 {
        // Just enough voltage to be at activation_threshold, triggering burst
        graph.neurons[i as usize].current_voltage = graph.neurons[i as usize].activation_threshold + 1.0;
    }
    
    graph.tick(1, env.clone());
    
    let mut spike_count = 0;
    for i in 0..20 {
        if graph.neurons[i as usize].last_spike_tick == 1 {
            spike_count += 1;
        }
    }
    
    println!("Spike count: {}", spike_count);
    assert_eq!(spike_count, 20, "All neurons in minicolumn must spike (Bursting) because none were predictive");
}

