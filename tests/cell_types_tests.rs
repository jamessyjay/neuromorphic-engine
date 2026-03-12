use neuromorphic_engine::graph::NetworkGraph;
use neuromorphic_engine::ffi::{Environment, NeuronType, Synapse};

fn default_env() -> Environment {
    Environment {
        global_oxygen_level: 1.0,
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0,
        toxin_levels: 0.0,
        light_level: 0.0,
        seed: 42,
    }
}

// 26. Test "Pacemaker Rhythm"
#[test]
fn test_pacemaker_rhythm() {
    let mut graph = NetworkGraph::new();
    let id = graph.add_neuron(NeuronType::Pacemaker);
    let env = default_env();
    
    // Give it a lot of ATP so it doesn't just sleep
    graph.neurons[id as usize].energy_atp = 1000.0;
    
    let mut spikes = 0;
    for tick in 1..=200 {
        graph.tick(tick, env.clone());
        let curr_v = graph.neurons[id as usize].current_voltage;
        
        if curr_v > 39.0 { // ACTION_POTENTIAL_PEAK is 40.0
            spikes += 1;
        }
    }
    
    assert!(spikes > 1, "Pacemaker should spike rhythmically on its own. Spikes: {}", spikes);
}

// 27. Test "Photoreceptor Dark Current"
#[test]
fn test_photoreceptor_dark_current() {
    let mut graph = NetworkGraph::new();
    let id = graph.add_neuron(NeuronType::Photoreceptor);
    let mut env = default_env();
    env.light_level = 0.0; // Total darkness
    
    for tick in 1..=100 {
        graph.tick(tick, env.clone());
    }
    
    let v = graph.neurons[id as usize].current_voltage;
    // Dark current should depolarize it towards -40mV
    assert!(v > -45.0 && v < -35.0, "Photoreceptor in dark should depolarize to ~-40mV. Actual: {}", v);
}

// 28. Test "Photoreceptor Light Bleach"
#[test]
fn test_photoreceptor_light_bleach() {
    let mut graph = NetworkGraph::new();
    let id = graph.add_neuron(NeuronType::Photoreceptor);
    let mut env = default_env();
    env.light_level = 1.0; // Max light
    
    for tick in 1..=100 {
        graph.tick(tick, env.clone());
    }
    
    let v = graph.neurons[id as usize].current_voltage;
    // Light bleach should hyperpolarize it to ~-65mV
    assert!(v > -70.0 && v < -60.0, "Photoreceptor in light should hyperpolarize to ~-65mV. Actual: {}", v);
}

// 29. Test "Nociceptor Sensitization"
#[test]
fn test_nociceptor_sensitization() {
    let mut graph = NetworkGraph::new();
    let id = graph.add_neuron(NeuronType::Nociceptor);
    let env = default_env();
    
    let initial_threshold = graph.neurons[id as usize].activation_threshold;
    
    for tick in 1..=50 {
        // Nociceptor sensitizes when it tries to spike but doesn't have ATP, or during normal updates
        // Let's just tick it with normal voltage
        graph.tick(tick, env.clone());
    }
    
    let final_threshold = graph.neurons[id as usize].activation_threshold;
    println!("Init: {}, Final: {}", initial_threshold, final_threshold);
    assert!(final_threshold < initial_threshold, "Nociceptor threshold should drop (sensitize) over time");
}

// 30. Test "Purkinje Output Integrity"
#[test]
fn test_purkinje_output_integrity() {
    let mut graph = NetworkGraph::new();
    let src = graph.add_neuron(NeuronType::Purkinje);
    let tgt = graph.add_neuron(NeuronType::Pyramidal); // Use pyramidal as motor placeholder since Motor doesn't exist
    let env = default_env();
    
    graph.synapses.push(Synapse {
        source_id: src,
        target_id: tgt,
        weight: 1.0,
        neurotransmitter_buffer: 0.0,
        last_activation_tick: 0,
        ticks_below_threshold: 0,
        ticks_until_delivery: 0,
        flags: 0,
        _padding: [0; 2],
    });
    
    // In rest, Target shouldn't spike randomly
    for tick in 1..=50 {
        graph.tick(tick, env.clone());
        assert!(graph.neurons[tgt as usize].current_voltage < -50.0, "Target should remain quiet without input");
    }
}
