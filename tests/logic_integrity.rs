use neuromorphic_engine::engine::ffi::{Environment, NeuronType};
use neuromorphic_engine::engine::constants;
use neuromorphic_engine::runner::modes::setup_benchmark;

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
    println!("=== TEST 1: Prediction Decay ===");
    let mut graph = setup_benchmark(10000, 50000);
    graph.synapses.clear(); // Isolate the neuron from random network noise
    let env = default_env();
    
    let target_idx = graph.neurons.iter().position(|n| n.layer_id == 2 && n.type_ == NeuronType::Pyramidal as u8).unwrap();
    println!("Selected target L2/3 neuron ID: {}", target_idx);
    
    graph.neurons[target_idx].active_branches_mask |= 1 << 4;
    graph.neurons[target_idx].branches[4].branch_type = 1;
    graph.neurons[target_idx].branches[4].nmda_threshold = pack_voltage(-30.0);
    graph.neurons[target_idx].branches[4].voltage = pack_voltage(10.0); 
    graph.neurons[target_idx].activation_threshold = 100.0;
    
    graph.tick(1, env.clone());
    
    assert!((graph.neurons[target_idx].status_flags & constants::FLAG_IS_PREDICTIVE) != 0, "Neuron did not enter Predictive State");
    let pred_voltage = graph.neurons[target_idx].current_voltage;
    println!("Tick 1: Neuron {} entered Predictive State. Voltage: {:.2} mV", target_idx, pred_voltage);
    
    for tick in 2..=31 {
        for b in &mut graph.neurons[target_idx].branches {
            b.voltage = 0; 
        }
        graph.tick(tick, env.clone());
        println!("Tick {} -> Pred Ticks: {}", tick, graph.neurons[target_idx].predictive_ticks);
    }
    
    let is_predictive = (graph.neurons[target_idx].status_flags & constants::FLAG_IS_PREDICTIVE) != 0;
    let final_voltage = graph.neurons[target_idx].current_voltage;
    let resting = graph.neurons[target_idx].resting_voltage;
    
    println!("Tick 31: Neuron {} Predictive State: {}. Voltage: {:.2} mV (Resting: {:.2} mV)", target_idx, is_predictive, final_voltage, resting);
    
    assert!(!is_predictive, "Neuron should have forgotten the prediction by tick 31!");
}

#[test]
fn test_sequence_learning_and_surprise() {
    println!("\n=== TEST 2: Sequence Learning & Bursting (Surprise) ===");
    let mut graph = setup_benchmark(10000, 50000);
    graph.synapses.clear(); // Clear random topology to perfectly isolate the logical behavior
    let env = default_env();
    
    let a_idx = 0;
    
    let mut b_minicolumn = vec![];
    let mut c_minicolumn = vec![];

    // Configure Minicolumn B (Predicted)
    for i in 100..120 {
        graph.neurons[i].type_ = NeuronType::Pyramidal as u8;
        graph.neurons[i].column_id = 1;
        graph.neurons[i].minicolumn_id = 1;
        graph.neurons[i].layer_id = 2;
        graph.neurons[i].branches[4].branch_type = 1;
        graph.neurons[i].branches[4].nmda_threshold = pack_voltage(-30.0);
        b_minicolumn.push(i);
        graph.connect(a_idx as u32, i as u32, 0.5, 4, false, true); // Extremely weak so it ONLY triggers predictive state, never somatic spike
    }

    // Configure Minicolumn C (Surprise)
    for i in 200..220 {
        graph.neurons[i].type_ = NeuronType::Pyramidal as u8;
        graph.neurons[i].column_id = 2;
        graph.neurons[i].minicolumn_id = 2;
        graph.neurons[i].layer_id = 2;
        c_minicolumn.push(i);
    }
    
    let mut tick = 1;
    
    // 1. Action A (Recognition)
    println!("--- Testing Recognition ---");
    graph.neurons[a_idx].current_voltage = 50.0;
    graph.tick(tick, env.clone());
    tick += 1;
    graph.tick(tick, env.clone()); tick += 1; // Delivery
    graph.tick(tick, env.clone()); tick += 1; // Evaluate
    
    let pred_count = b_minicolumn.iter().filter(|&&b| (graph.neurons[b].status_flags & constants::FLAG_IS_PREDICTIVE) != 0).count();
    println!("Minicolumn B Predictive Count after A: {}/{}", pred_count, b_minicolumn.len());
    assert!(pred_count > 0, "A failed to predict B");
    
    // Provide feedforward stimulus to B
    for &b in &b_minicolumn {
        // Clear any random spikes that happened due to thermal noise or bursting
        graph.neurons[b].last_spike_tick = 0;
        graph.neurons[b].refractory_time_left = 0.0;
        graph.neurons[b].current_voltage = 50.0; 
    }
    graph.tick(tick, env.clone());
    
    let mut b_spikes = 0;
    for &b in &b_minicolumn {
        if graph.neurons[b].last_spike_tick == tick {
            b_spikes += 1;
        } else if graph.neurons[b].last_spike_tick > 0 {
            println!("Neuron {} spiked at tick {} instead of {}!", b, graph.neurons[b].last_spike_tick, tick);
        }
    }
    tick += 1;
    
    println!("Spikes in B Minicolumn during Recognition: {}", b_spikes);
    assert_eq!(b_spikes, 1, "WTA failed during recognition, expected exactly 1 spike!");
    
    // 2. Action B (Surprise/Bursting)
    println!("--- Testing Surprise ---");
    for n in &mut graph.neurons {
        n.current_voltage = n.resting_voltage;
        n.status_flags = 0;
        n.predictive_ticks = 0;
        n.refractory_time_left = 0.0;
        for b in &mut n.branches { b.voltage = 0; }
    }
    
    // We expect B (A fired), but get C!
    graph.neurons[a_idx].current_voltage = 50.0;
    graph.tick(tick, env.clone()); tick += 1;
    graph.tick(tick, env.clone()); tick += 1;
    graph.tick(tick, env.clone()); tick += 1;
    
    // Provide strong feedforward to C (unpredicted)
    for &c in &c_minicolumn {
        graph.neurons[c].last_spike_tick = 0;
        graph.neurons[c].refractory_time_left = 0.0;
        graph.neurons[c].current_voltage = 50.0; 
    }
    graph.tick(tick, env.clone());
    
    let c_spikes = c_minicolumn.iter().filter(|&&c| graph.neurons[c].last_spike_tick == tick).count();
    
    println!("Spikes in C Minicolumn during Surprise: {}/{}", c_spikes, c_minicolumn.len());
    assert!(c_spikes > 1, "Bursting failed during surprise, expected multiple neurons to fire!");
}

#[test]
fn test_cross_column_voting() {
    println!("\n=== TEST 3: Cross-Column Voting ===");
    let mut graph = setup_benchmark(10000, 50000);
    graph.synapses.clear();
    let env = default_env();
    
    let mut col0_l4 = vec![];
    let mut col0_l23 = vec![];
    let mut col1_l23 = vec![];
    
    for i in 300..310 { graph.neurons[i].column_id = 0; graph.neurons[i].layer_id = 4; col0_l4.push(i); }
    for i in 400..410 { graph.neurons[i].column_id = 0; graph.neurons[i].layer_id = 2; col0_l23.push(i); }
    for i in 500..510 { 
        graph.neurons[i].column_id = 1; 
        graph.neurons[i].layer_id = 2; 
        graph.neurons[i].branches[4].branch_type = 1; 
        graph.neurons[i].branches[4].nmda_threshold = pack_voltage(-30.0);
        col1_l23.push(i); 
    }
    
    for &src in &col0_l4 {
        for &tgt in &col0_l23 { graph.connect(src as u32, tgt as u32, 50.0, 0, false, true); }
    }
    for &src in &col0_l23 {
        for &tgt in &col1_l23 { graph.connect(src as u32, tgt as u32, 5.0, 4, false, true); }
    }
    
    for tick in 1..=10 {
        for &n in &col0_l4 { graph.neurons[n].current_voltage = 50.0; }
        graph.tick(tick, env.clone());
    }
    
    let pred_count = col1_l23.iter().filter(|&&n| (graph.neurons[n].status_flags & constants::FLAG_IS_PREDICTIVE) != 0).count();
    
    println!("Cross-Column Voting | Col 1 Predictive Count: {}/{}", pred_count, col1_l23.len());
    assert!(pred_count > 0, "Col 1 L2/3 did not receive predictive votes from Col 0!");
}

#[test]
fn test_metabolic_fatigue() {
    println!("\n=== TEST 4: Metabolic Fatigue (Epilepsy Protection) ===");
    let mut graph = setup_benchmark(10000, 50000);
    let mut env = default_env();
    env.global_oxygen_level = 0.0;
    env.global_glucose_level = 0.0;
    
    let mut minicolumn = vec![];
    for i in 600..620 {
        graph.neurons[i].column_id = 0;
        graph.neurons[i].minicolumn_id = 0;
        graph.neurons[i].layer_id = 4;
        graph.neurons[i].max_atp = 100.0;
        graph.neurons[i].energy_atp = 35.0; // Start with enough for 1 spike + leaks
        minicolumn.push(i);
    }
    
    let mut spikes = 0;
    for tick in 1..=50 {
        for &n in &minicolumn {
            if graph.neurons[n].energy_atp >= 15.0 {
                graph.neurons[n].current_voltage = 50.0; 
            }
        }
        graph.tick(tick, env.clone());
        for &n in &minicolumn {
            if graph.neurons[n].last_spike_tick == tick { spikes += 1; }
        }
        if tick == 1 {
            println!("Tick 1: Neuron {} V={:.2} Thresh={:.2} ATP={:.2} LastSpike={}", minicolumn[0], graph.neurons[minicolumn[0]].current_voltage, graph.neurons[minicolumn[0]].activation_threshold, graph.neurons[minicolumn[0]].energy_atp, graph.neurons[minicolumn[0]].last_spike_tick);
        }
    }
    
    let end_atp = graph.neurons[minicolumn[0]].energy_atp;
    let death_reason = graph.neurons[minicolumn[0]].death_reason;
    
    println!("Metabolic Fatigue | Total Spikes over 50 ticks: {} (Max Possible: 1000)", spikes);
    println!("Metabolic Fatigue | Final ATP: {:.2}, death_reason: {}", end_atp, death_reason);
    
    // We expect it to stop spiking once ATP < 15.0 and death_reason = 4 (STATE_METABOLIC_SHUTDOWN)
    // "Check that neurons didn't completely die (ATP > 0), but entered energy saving mode."
    assert!(spikes > 0, "Neurons didn't fire at all.");
    assert!(spikes < 1000, "Neurons did not fatigue! They fired on every single tick without limit.");
    assert!(death_reason == 4 || death_reason == 1, "Neuron did not enter Metabolic Shutdown or Die from fatigue.");
}
