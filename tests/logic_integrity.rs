use neuromorphic_engine::network::graph::NetworkGraph;
use neuromorphic_engine::engine::ffi::{Environment, NeuronType};
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
    }
}

fn pack_voltage(v: f32) -> u16 {
    ((v + 100.0) * 100.0).clamp(0.0, 65535.0) as u16
}

#[test]
fn test_prediction_decay() {
    let mut graph = NetworkGraph::new();
    let env = default_env();
    
    // 1. Setup: Create a single isolated L2/3 neuron to avoid network noise
    let target_idx = graph.add_neuron(NeuronType::Pyramidal) as usize;
    graph.neurons[target_idx].layer_id = 2;
    
    graph.neurons[target_idx].branches[4].branch_type = 1;
    graph.neurons[target_idx].branches[4].nmda_threshold = pack_voltage(-30.0);
    graph.neurons[target_idx].branches[4].voltage = pack_voltage(-10.0);
    // Prevent the neuron from actually spiking during the decay test
    graph.neurons[target_idx].activation_threshold = 100.0;
    
    graph.tick(1, env.clone());
    
    assert!((graph.neurons[target_idx].status_flags & constants::FLAG_IS_PREDICTIVE) != 0, "Neuron did not enter Predictive State");
    let pred_voltage = graph.neurons[target_idx].current_voltage;
    
    for t in 2..=31 {
        graph.tick(t, env.clone());
    }
    
    let n = &graph.neurons[target_idx];
    assert!((n.status_flags & constants::FLAG_IS_PREDICTIVE) == 0, "Prediction did not decay after 20 ticks!");
    assert!(n.current_voltage < pred_voltage, "Voltage did not repolarize after prediction decay!");
}

#[test]
fn test_sequence_learning_and_surprise() {
    let mut graph = NetworkGraph::new();
    let env = default_env();
    
    let a_idx = graph.add_neuron(NeuronType::Pyramidal) as usize;
    graph.neurons[a_idx].column_id = 0;
    
    let mut b_minicolumn = Vec::new();
    for _ in 0..20 {
        let n = graph.add_neuron(NeuronType::Pyramidal) as usize;
        graph.neurons[n].column_id = 1;
        graph.neurons[n].minicolumn_id = 1;
        graph.neurons[n].layer_id = 2;
        graph.neurons[n].branches[4].branch_type = 1;
        graph.neurons[n].branches[4].nmda_threshold = pack_voltage(-30.0);
        b_minicolumn.push(n);
    }
    
    let mut c_minicolumn = Vec::new();
    for _ in 0..20 {
        let n = graph.add_neuron(NeuronType::Pyramidal) as usize;
        graph.neurons[n].column_id = 2;
        graph.neurons[n].minicolumn_id = 2;
        graph.neurons[n].layer_id = 2;
        graph.neurons[n].branches[4].branch_type = 1;
        graph.neurons[n].branches[4].nmda_threshold = pack_voltage(-30.0);
        c_minicolumn.push(n);
    }
    
    for &b in &b_minicolumn {
        graph.connect(a_idx as u32, b as u32, 10.0, 4, false, false);
    }
    
    // Test 1: Recognition (A -> B)
    graph.neurons[a_idx].current_voltage = 50.0;
    graph.tick(1, env.clone());
    
    graph.tick(2, env.clone());
    graph.tick(3, env.clone());
    
    let mut pred_count = 0;
    for &b in &b_minicolumn {
        if (graph.neurons[b].status_flags & constants::FLAG_IS_PREDICTIVE) != 0 {
            pred_count += 1;
        }
    }
    assert!(pred_count > 0, "A failed to predict B");
    
    for &b in &b_minicolumn {
        graph.neurons[b].current_voltage = graph.neurons[b].activation_threshold + 5.0;
    }
    
    let pre_atp_b: f32 = b_minicolumn.iter().map(|&i| graph.neurons[i].energy_atp).sum();
    graph.tick(4, env.clone());
    let post_atp_b: f32 = b_minicolumn.iter().map(|&i| graph.neurons[i].energy_atp).sum();
    let cost_recognition = pre_atp_b - post_atp_b;
    
    let mut b_spikes = 0;
    for &b in &b_minicolumn {
        if graph.neurons[b].last_spike_tick == 4 {
            b_spikes += 1;
        }
    }
    
    assert_eq!(b_spikes, 1, "WTA failed during recognition, expected exactly 1 spike!");
    
    // Clear states for surprise test
    for n in &mut graph.neurons {
        n.current_voltage = n.resting_voltage;
        n.status_flags = 0;
        n.predictive_ticks = 0;
        n.refractory_time_left = 0.0;
        for b in &mut n.branches {
            b.voltage = pack_voltage(-100.0);
        }
    }
    
    // Test 2: Surprise (A -> C)
    graph.neurons[a_idx].current_voltage = 50.0;
    graph.tick(10, env.clone());
    graph.tick(11, env.clone());
    graph.tick(12, env.clone());
    
    for &c in &c_minicolumn {
        // High enough voltage so it doesn't decay below threshold during `tick_neuron_prepare_cpu`
        graph.neurons[c].current_voltage = graph.neurons[c].activation_threshold + 30.0;
    }
    
    let pre_atp_c: f32 = c_minicolumn.iter().map(|&i| graph.neurons[i].energy_atp).sum();
    graph.tick(13, env.clone());
    let post_atp_c: f32 = c_minicolumn.iter().map(|&i| graph.neurons[i].energy_atp).sum();
    let cost_surprise = pre_atp_c - post_atp_c;
    
    let mut c_spikes = 0;
    for &c in &c_minicolumn {
        if graph.neurons[c].last_spike_tick == 13 {
            c_spikes += 1;
        }
    }
    
    assert_eq!(c_spikes, 20, "Bursting failed during surprise, expected all 20 neurons to fire!");
    assert!(cost_surprise > cost_recognition * 10.0, "Surprise should cost significantly more ATP than recognition!");
}

#[test]
fn test_cross_column_voting() {
    let mut graph = NetworkGraph::new();
    let env = default_env();
    
    // Create L4 input for Col 0
    let mut col0_l4 = Vec::new();
    for _ in 0..10 {
        let n = graph.add_neuron(NeuronType::Pyramidal) as usize;
        graph.neurons[n].column_id = 0;
        graph.neurons[n].layer_id = 4;
        col0_l4.push(n);
    }
    
    // Create L2/3 for Col 0
    let mut col0_l23 = Vec::new();
    for _ in 0..10 {
        let n = graph.add_neuron(NeuronType::Pyramidal) as usize;
        graph.neurons[n].column_id = 0;
        graph.neurons[n].layer_id = 2;
        col0_l23.push(n);
    }
    
    // Create L2/3 for Col 1
    let mut col1_l23 = Vec::new();
    for _ in 0..10 {
        let n = graph.add_neuron(NeuronType::Pyramidal) as usize;
        graph.neurons[n].column_id = 1;
        graph.neurons[n].layer_id = 2;
        graph.neurons[n].branches[4].branch_type = 1; // Distal
        graph.neurons[n].branches[4].nmda_threshold = pack_voltage(-30.0);
        col1_l23.push(n);
    }
    
    // Connect Col 0 L4 -> Col 0 L2/3 (Feedforward)
    for &src in &col0_l4 {
        for &tgt in &col0_l23 {
            graph.connect(src as u32, tgt as u32, 10.0, 0, false, false); // Branch 0 (Proximal)
        }
    }
    
    // Connect Col 0 L2/3 -> Col 1 L2/3 (Horizontal Voting)
    for &src in &col0_l23 {
        for &tgt in &col1_l23 {
            graph.connect(src as u32, tgt as u32, 5.0, 4, false, false); // Branch 4 (Distal)
        }
    }
    
    // Action: Fire L4 of Col 0 heavily for a few ticks
    for tick in 1..=10 {
        for &n in &col0_l4 {
            graph.neurons[n].current_voltage = 50.0;
        }
        graph.tick(tick, env.clone());
    }
    
    // Check Col 1 L2/3. Did they get the vote?
    let mut pred_count = 0;
    for &n in &col1_l23 {
        if (graph.neurons[n].status_flags & constants::FLAG_IS_PREDICTIVE) != 0 {
            pred_count += 1;
        }
    }
    
    println!("Cross-Column Voting | Col 1 Predictive Count: {}/{}", pred_count, col1_l23.len());
    assert!(pred_count > 0, "Col 1 L2/3 did not receive predictive votes from Col 0!");
}

#[test]
fn test_metabolic_fatigue() {
    let mut graph = NetworkGraph::new();
    let mut env = default_env();
    env.global_glucose_level = 0.0; // Starve them so they can't recover ATP
    env.global_oxygen_level = 0.0;
    
    // Create a minicolumn of 20 neurons
    let mut minicolumn = Vec::new();
    for _ in 0..20 {
        let n = graph.add_neuron(NeuronType::Pyramidal) as usize;
        graph.neurons[n].column_id = 0;
        graph.neurons[n].minicolumn_id = 0;
        graph.neurons[n].layer_id = 4; // L4 to receive input
        
        // Give them a normal ATP
        graph.neurons[n].max_atp = 100.0;
        graph.neurons[n].energy_atp = 100.0;
        
        minicolumn.push(n);
    }
    
    // We will force bursting for 50 ticks by providing unexpected, strong signals 
    // to random neurons in the minicolumn to prevent WTA suppression from saving them.
    // Wait, if we provide stimulus to ALL of them, they will all burst.
    // Bursting costs ATP_SPIKE_COST (usually 10 or 20) per spike.
    // 50 ticks of bursting = 50 spikes * 20 ATP = 1000 ATP. 
    // Max ATP is 100. They will definitely run out and become fatigued.
    
    let mut spikes = 0;
    
    for tick in 1..=50 {
        for &n in &minicolumn {
            // Provide a mild stimulus. When ATP is full, this crosses the threshold (-50mV approx).
            // When ATP drops, dynamic threshold rises to say -30mV, and this stimulus won't trigger a spike anymore.
            graph.neurons[n].current_voltage = graph.neurons[n].activation_threshold + 5.0;
        }
        
        graph.tick(tick, env.clone());
        
        for &n in &minicolumn {
            if graph.neurons[n].last_spike_tick == tick {
                spikes += 1;
            }
        }
    }
    
    let end_atp = graph.neurons[minicolumn[0]].energy_atp;
    let end_voltage = graph.neurons[minicolumn[0]].current_voltage;
    
    println!("Metabolic Fatigue | Total Spikes over 50 ticks: {} (Max Possible: 1000)", spikes);
    println!("Metabolic Fatigue | Final ATP: {:.2}, Final Voltage: {:.2}", end_atp, end_voltage);
    
    assert!(spikes < 1000, "Neurons did not fatigue! They fired on every single tick without limit.");
    assert!(spikes > 0, "Neurons didn't fire at all.");
    assert!(end_atp < 80.0, "ATP was not depleted by bursting.");
    assert!(end_voltage < -40.0, "Neuron did not repolarize / ignore stimulus after fatigue.");
}
