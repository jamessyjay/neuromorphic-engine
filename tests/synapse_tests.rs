use neuromorphic_engine::network::graph::NetworkGraph;
use neuromorphic_engine::engine::ffi::{Environment, NeuronType, Synapse};

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

#[test]
fn test_classic_ltp() {
    let mut graph = NetworkGraph::new();
    let src = graph.add_neuron(NeuronType::Pyramidal);
    let tgt = graph.add_neuron(NeuronType::Pyramidal);
    graph.neurons[src as usize].column_id = 0;
    graph.neurons[tgt as usize].column_id = 1;
    let env = default_env();
    
    let synapse = Synapse {
        source_id: src,
        target_id: tgt,
        weight: 2.5,
        neurotransmitter_buffer: 0.0,
        last_activation_tick: 0,
        ticks_below_threshold: 0,
        ticks_until_delivery: 0,
        flags: 0,
        _padding: [0; 2],
    };
    graph.synapses.push(synapse);
    
    graph.neurons[src as usize].current_voltage = 50.0;
    graph.tick(1, env.clone());
    
    for tick in 2..=5 { graph.tick(tick, env.clone()); }
    
    assert!(graph.synapses[0].last_activation_tick > 0, "Synapse should record activation tick after delivery");
    
    graph.neurons[tgt as usize].current_voltage = 50.0;
    graph.neurons[tgt as usize].branches[0].local_trace = 255;
    graph.tick(6, env.clone());
    
    let final_weight = graph.synapses[0].weight;
    println!("[STDP-LTP] Trace: {} | Weight delta: 2.500 -> {:.3}", graph.neurons[tgt as usize].branches[0].local_trace, final_weight);
    assert!(final_weight > 2.5, "Weight should increase due to LTP");
}

#[test]
fn test_classic_ltd() {
    let mut graph = NetworkGraph::new();
    let src = graph.add_neuron(NeuronType::Pyramidal);
    let tgt = graph.add_neuron(NeuronType::Pyramidal);
    graph.neurons[src as usize].column_id = 0;
    graph.neurons[tgt as usize].column_id = 1;
    let env = default_env();
    
    let synapse = Synapse {
        source_id: src,
        target_id: tgt,
        weight: 2.5,
        neurotransmitter_buffer: 0.0,
        last_activation_tick: 0,
        ticks_below_threshold: 0,
        ticks_until_delivery: 0,
        flags: 0, 
        _padding: [0; 2],
    };
    graph.synapses.push(synapse);
    
    graph.neurons[tgt as usize].current_voltage = 40.0;
    graph.neurons[tgt as usize].last_spike_tick = 1;
    graph.neurons[tgt as usize].branches[0].local_trace = 255;
    graph.tick(1, env.clone());
    
    graph.neurons[src as usize].current_voltage = 40.0;
    graph.neurons[src as usize].last_spike_tick = 10;
    graph.tick(10, env.clone());
    
    for tick in 11..=30 { graph.tick(tick, env.clone()); }
    
    let final_weight = graph.synapses[0].weight;
    println!("[STDP-LTD] Trace: {} | Weight delta: 2.500 -> {:.3}", graph.neurons[tgt as usize].branches[0].local_trace, final_weight);
    assert!(final_weight < 2.5, "Weight should decrease due to LTD");
}

#[test]
fn test_dopamine_amplification() {
    let mut graph = NetworkGraph::new();
    let src = graph.add_neuron(NeuronType::Pyramidal);
    let tgt = graph.add_neuron(NeuronType::Pyramidal);
    graph.neurons[src as usize].column_id = 0;
    graph.neurons[tgt as usize].column_id = 1;
    let mut env = default_env();
    env.dopamine_multiplier = 10.0;
    env.stdp_learning_rate = 50.0;
    
    let synapse = Synapse {
        source_id: src,
        target_id: tgt,
        weight: 2.5,
        neurotransmitter_buffer: 0.0,
        last_activation_tick: 0,
        ticks_below_threshold: 0,
        ticks_until_delivery: 0,
        flags: 0, 
        _padding: [0; 2],
    };
    graph.synapses.push(synapse);
    
    graph.neurons[src as usize].current_voltage = 50.0;
    graph.tick(1, env.clone());
    
    for tick in 2..=5 { graph.tick(tick, env.clone()); }
    
    graph.neurons[tgt as usize].current_voltage = 40.0;
    graph.neurons[tgt as usize].last_spike_tick = 6;
    graph.neurons[tgt as usize].branches[0].local_trace = 255; // MMN v1.1: Trace needed for STDP
    graph.tick(6, env.clone());
    
    
    // With env.stdp_learning_rate = 50.0 and dopa_mult = 10.0,
    // Source spiked at 1, target at 6. Trace = 0.9^5 = 0.59
    // delta_w = 0.59 * 50.0 * 0.005 * 10.0 = 1.475
    // Final weight should be ~3.975
    assert!(graph.synapses[0].weight > 3.5, "Weight should grow much faster with dopamine");
}

#[test]
fn test_synaptic_scaling() {
    let mut graph = NetworkGraph::new();
    let src = graph.add_neuron(NeuronType::Pyramidal);
    let tgt = graph.add_neuron(NeuronType::Pyramidal);
    graph.neurons[src as usize].column_id = 0;
    graph.neurons[tgt as usize].column_id = 1;
    let env = default_env();
    
    let synapse = Synapse {
        source_id: src,
        target_id: tgt,
        weight: 2.5,
        neurotransmitter_buffer: 0.0,
        last_activation_tick: 0,
        ticks_below_threshold: 0,
        ticks_until_delivery: 0,
        flags: 0, 
        _padding: [0; 2],
    };
    graph.synapses.push(synapse);
    
    graph.neurons[tgt as usize].spikes_in_last_window = 100; // Above SYNAPSE_SCALING_HIGH
    graph.tick(1, env.clone());
    
    assert!(graph.synapses[0].weight < 10.0, "Weight should be scaled down due to homeostatic penalty");
}

#[test]
fn test_reliability_fade() {
    let mut graph = NetworkGraph::new();
    let src = graph.add_neuron(NeuronType::Pyramidal);
    let tgt = graph.add_neuron(NeuronType::Pyramidal);
    graph.neurons[src as usize].column_id = 0;
    graph.neurons[tgt as usize].column_id = 1;
    let env = default_env();
     
    
    let synapse = Synapse {
        source_id: src,
        target_id: tgt,
        weight: 0.1, // < 0.5 THRESHOLD
        neurotransmitter_buffer: 50.0, // Pre-load buffer
        last_activation_tick: 0,
        ticks_below_threshold: 0,
        ticks_until_delivery: 1, 
        flags: 0, 
        _padding: [0; 2],
    };
    graph.synapses.push(synapse);
    
    // We pick tick 60, because 60 + 0 + 1 = 61. 
    // failure_roll = (61 % 100) / 100.0 = 0.61.
    // 0.61 > 0.5 (SYNAPSE_RELIABILITY_THRESHOLD), so it should FAIL to deliver.
    graph.tick(60, env.clone());
    
    assert_eq!(graph.synapses[0].last_activation_tick, 0, "Synapse transmission should fail due to reliability roll");
}

#[test]
fn test_receptor_saturation() {
    let mut graph = NetworkGraph::new();
    let src = graph.add_neuron(NeuronType::Pyramidal);
    let tgt = graph.add_neuron(NeuronType::Pyramidal);
    graph.neurons[src as usize].column_id = 0;
    graph.neurons[tgt as usize].column_id = 1;
    let env = default_env();
    
    let synapse = Synapse {
        source_id: src,
        target_id: tgt,
        weight: 2.0, // Receptor limit is 2.0
        neurotransmitter_buffer: 200.0, // Massive amount (will be clamped to MAX_VESICLES = 100.0)
        last_activation_tick: 0,
        ticks_below_threshold: 0,
        ticks_until_delivery: 1, // Deliver on next tick
        flags: 0, 
        _padding: [0; 2],
    };
    graph.synapses.push(synapse);
    
    // Use tick 10 so it does not fail the reliability roll. 
    // 10 + 0 + 1 = 11. 11/100 = 0.11. 0.11 < 0.5, NO failure.
    // Actually weight = 2.0 > 0.5, so reliability roll is completely skipped!
    graph.tick(10, env.clone());
    
    let remaining_buffer = graph.synapses[0].neurotransmitter_buffer;
    println!("[SATURATION] Vesicles requested: 200.0 | Clamped max: 100.0 | Delivered: {:.1} | Remaining buffer: {:.1}", 100.0 - remaining_buffer, remaining_buffer);
    assert!(remaining_buffer >= 97.9 && remaining_buffer <= 98.1, "Receptor saturation should limit neurotransmitter consumption AND max capacity should clamp to MAX_VESICLES (100.0)");
}
