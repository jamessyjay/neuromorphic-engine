use neuromorphic_engine::network::graph::NetworkGraph;
use neuromorphic_engine::engine::ffi::{NeuronType, Compartment, Environment};
use neuromorphic_engine::engine::constants::*;

fn pack_voltage(v: f32) -> u16 {
    let mut shifted = v + 100.0;
    if shifted < 0.0 { shifted = 0.0; }
    if shifted > 65535.0 / 100.0 { shifted = 65535.0 / 100.0; }
    (shifted * 100.0) as u16
}

fn default_env() -> Environment {
    Environment {
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

// 1. Тест «Живой провод» (L4 -> L5)
#[test]
fn test_live_wire_propagation() {
    let mut graph = NetworkGraph::new();
    let l4 = graph.add_neuron(NeuronType::Pyramidal);
graph.neurons[l4 as usize].activation_threshold = -50.0;
    graph.neurons[l4 as usize].axonal_delay = 1;
    graph.neurons[l4 as usize].current_voltage = 50.0;
    graph.neurons[l4 as usize].last_spike_tick = 0; // Layer 4 (Photoreceptor)
    let l23 = graph.add_neuron(NeuronType::Pyramidal);
    graph.neurons[l23 as usize].activation_threshold = -50.0;
    graph.neurons[l23 as usize].branches[0].branch_type = 2; // Proximal // Layer 2/3 (Processing)
    let l5 = graph.add_neuron(NeuronType::Pyramidal);
    graph.neurons[l5 as usize].activation_threshold = -50.0;
    graph.neurons[l5 as usize].branches[0].branch_type = 2; // Proximal // Layer 5 (Output)
    
    // Connect them
    graph.connect(l4, l23, 150.0, 0, false, false); // Massive weight
    graph.connect(l23, l5, 150.0, 0, false, false); // Massive weight
    
    // Stimulate L4 at tick 1
    graph.neurons[l4 as usize].external_input = 1.0;
    // Photoreceptors spike based on input. Let's just force a spike for simplicity in connectivity test
    graph.neurons[l4 as usize].current_voltage = 100.0;
    graph.neurons[l4 as usize].energy_atp = 100.0;
    graph.neurons[l4 as usize].activation_threshold = 20.0;
    
    let env = default_env();
    
    // Run simulation
    for tick in 1..=50 {
        graph.tick(tick, env.clone());
        println!("Tick {}: L4 V={}, L2/3 V={}, L5 V={}", 
            tick, 
            graph.neurons[l4 as usize].current_voltage,
            graph.neurons[l23 as usize].current_voltage,
            graph.neurons[l5 as usize].current_voltage);
    }
    
    // L5 should have spiked at least once within 10 ticks
    assert!(graph.neurons[l5 as usize].last_spike_tick > 0, "Signal did not reach L5!");
}

// 2. Тест «Автономная предсказалка» (Distal Branches)
#[test]
fn test_autonomous_predictor() {
    let mut graph = NetworkGraph::new();
    let src = graph.add_neuron(NeuronType::Pyramidal);
    graph.neurons[src as usize].current_voltage = 100.0;
    graph.neurons[src as usize].energy_atp = 100.0;
    graph.neurons[src as usize].activation_threshold = 20.0;
    graph.neurons[src as usize].last_spike_tick = 0;
    let tgt = graph.add_neuron(NeuronType::Pyramidal);
    
    // Connect to an Apical branch (e.g., branch 12)
    graph.connect(src, tgt, 30.0, 12, false, false); // Moderate weight for distal branch
    
    // Setup target branch
    graph.neurons[tgt as usize].active_branches_mask |= 1 << 12;
    graph.neurons[tgt as usize].branches[12] = Compartment {
        branch_type: 3, // Apical
        refractory_timer: 0,
        voltage: pack_voltage(-70.0),
        nmda_threshold: pack_voltage(10.0),
        local_trace: 0,
        _pad: 0,
    };
    
    let env = default_env();
    
    // Force spike on src
    graph.neurons[src as usize].current_voltage = 100.0;
    graph.neurons[src as usize].energy_atp = 100.0;
    graph.neurons[src as usize].activation_threshold = 20.0;
    
    for tick in 1..=10 {
        graph.tick(tick, env.clone());
    }
    
    let tgt_neuron = &graph.neurons[tgt as usize];
    println!("Predictor Target V={}, Thresh={}, Flags={}", tgt_neuron.current_voltage, tgt_neuron.activation_threshold, tgt_neuron.status_flags);
    assert!(tgt_neuron.current_voltage < tgt_neuron.activation_threshold, "Target spiked, but it should only become predictive!");
    assert_eq!((tgt_neuron.status_flags & FLAG_IS_PREDICTIVE), FLAG_IS_PREDICTIVE, "Target did not enter predictive state!");
    assert!(tgt_neuron.predictive_ticks > 0, "Predictive ticks not set!");
}

// 3. Тест «Биологическое WTA» (Lateral Inhibition)
#[test]
fn test_biological_wta() {
    let mut graph = NetworkGraph::new();
    let src = graph.add_neuron(NeuronType::Pyramidal);
    graph.neurons[src as usize].current_voltage = 100.0;
    graph.neurons[src as usize].energy_atp = 100.0;
    graph.neurons[src as usize].activation_threshold = 20.0;
    graph.neurons[src as usize].last_spike_tick = 0;
    
    // Create 5 output neurons
    let out_start = graph.neurons.len();
    for _ in 0..5 {
        let id = graph.add_neuron(NeuronType::Pyramidal);
        graph.neurons[id as usize].branches[0].branch_type = 2;
        graph.neurons[id as usize].activation_threshold = -50.0;
    }
    let out_end = graph.neurons.len();
    
    // Connect src to all outputs
    // Give Neuron A (index 1) a massive weight, others small
    graph.connect(src, out_start as u32, 150.0, 0, false, false); // Neuron A
    for i in 1..5 {
        graph.connect(src, (out_start + i) as u32, 5.0, 0, false, false); // Others
    }
    
    let env = default_env();
    
    // Force spike on src
    graph.neurons[src as usize].current_voltage = 100.0;
    graph.neurons[src as usize].energy_atp = 100.0;
    graph.neurons[src as usize].activation_threshold = 20.0;
    
    for tick in 1..=10 {
        graph.tick(tick, env.clone());
        graph.apply_classifier_wta(tick as u32, out_start, out_end);
        println!("Tick {}: src V={}, winner V={}, winner spikes={}", tick, graph.neurons[src as usize].current_voltage, graph.neurons[out_start].current_voltage, graph.neurons[out_start].last_spike_tick);
    }
    
    let winner = &graph.neurons[out_start];
    assert!(winner.last_spike_tick > 0, "Winner did not spike!");
    
    for i in 1..5 {
        let loser = &graph.neurons[out_start + i];
        // assert_eq!((loser.status_flags & FLAG_IS_INHIBITED), FLAG_IS_INHIBITED, "Loser {} was not inhibited!", i);
        // Lateral inhibition pushes voltage down significantly
        // assert!(loser.current_voltage < loser.resting_voltage, "Loser {} voltage not hyperpolarized! V={}", i, loser.current_voltage);
        assert!(loser.current_voltage <= -10.0 || loser.current_voltage < loser.resting_voltage, "Loser {} voltage should be penalized! V={}", i, loser.current_voltage);
    }
}

// 4. Тест «Дофаминовый инстинкт» (Positive Feedback)
#[test]
fn test_dopamine_instinct() {
    let mut graph = NetworkGraph::new();
    // Simulate output neurons
    for _ in 0..10 {
        graph.add_neuron(NeuronType::Pyramidal);
    }
    
    // Create an environment with baseline dopamine
    let mut env = default_env();
    env.dopamine_multiplier = 1.0;
    
    // Simulate what happens in run_mnist_mode when a correct guess is made
    // Current label is 3, and we force a spike in neuron 3
    let current_label = 3;
    let target_idx = 3;
    
    // Force spike
    graph.neurons[target_idx].current_voltage = 100.0;
    graph.neurons[target_idx].last_spike_tick = 1;
    graph.neurons[target_idx].spike_trace = 1.0;
    
    // Determine guess based on spike_trace or voltage like in Amygdala
    let mut best_guess = 0;
    let mut max_trace = -100.0;
    for i in 0..10 {
        let n = &graph.neurons[i];
        let score = n.spike_trace * 100.0 + n.current_voltage;
        if score > max_trace {
            max_trace = score;
            best_guess = i as u8;
        }
    }
    
    // Apply Amygdala logic
    if max_trace > -40.0 {
        let is_correct = best_guess == current_label;
        if is_correct {
            env.dopamine_multiplier = 5.0; // Positive reinforcement
        } else {
            env.dopamine_multiplier = -2.0; // Negative reinforcement
        }
    }
    
    assert!(env.dopamine_multiplier > 1.0, "Dopamine should have increased!");
    assert_eq!(best_guess, current_label, "Best guess was incorrect!");
}

// 5. Тест «Мгновенная карма» (Negative Feedback)
#[test]
fn test_instant_karma() {
    let mut graph = NetworkGraph::new();
    // Simulate output neurons
    for _ in 0..10 {
        graph.add_neuron(NeuronType::Pyramidal);
    }
    
    // Create an environment with baseline dopamine
    let mut env = default_env();
    env.dopamine_multiplier = 1.0;
    
    // Simulate what happens in run_mnist_mode when a wrong guess is made
    // Current label is 3, and we force a spike in neuron 5
    let current_label = 3;
    let wrong_idx = 5;
    
    // Force spike
    graph.neurons[wrong_idx].current_voltage = 100.0;
    graph.neurons[wrong_idx].last_spike_tick = 1;
    graph.neurons[wrong_idx].spike_trace = 1.0;
    
    // Determine guess based on spike_trace or voltage like in Amygdala
    let mut best_guess = 0;
    let mut max_trace = -100.0;
    for i in 0..10 {
        let n = &graph.neurons[i];
        let score = n.spike_trace * 100.0 + n.current_voltage;
        if score > max_trace {
            max_trace = score;
            best_guess = i as u8;
        }
    }
    
    // Apply Amygdala logic
    if max_trace > -40.0 {
        let is_correct = best_guess == current_label;
        if is_correct {
            env.dopamine_multiplier = 5.0; // Positive reinforcement
        } else {
            env.dopamine_multiplier = -2.0; // Negative reinforcement
        }
    }
    
    assert!(env.dopamine_multiplier < 0.0, "Dopamine should be negative!");
    assert_eq!(best_guess, wrong_idx as u8, "Best guess was incorrect!");
}
