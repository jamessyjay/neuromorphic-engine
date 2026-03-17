use neuromorphic_engine::network::graph::NetworkGraph;
use neuromorphic_engine::engine::ffi::{Environment, NeuronType};

#[test]
fn test_atomic_hammer() {
    println!("--- EXECUTING TEST 65: ATOMIC HAMMER (CUDA ATOMICS) ---");
    let mut graph = NetworkGraph::new();
    
    // 1 Target, 512 Sources
    graph.add_neuron(NeuronType::Pyramidal); // Target ID 0
    graph.neurons[0].internal_phase = 0; // Sync phase

    for _ in 0..512 {
        graph.add_neuron(NeuronType::Pyramidal);
        let last_idx = graph.neurons.len() - 1;
        graph.neurons[last_idx].internal_phase = 0; // Sync phase
    }
    
    let env = Environment {
        global_oxygen_level: 1.0,
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0,
        toxin_levels: 0.0,
        light_level: 1.0,
        stdp_learning_rate: 0.0, 
        wakefulness_multiplier: 1.0,
    };

    let branch_id = 1;
    // We want exactly 0.1f impact per synapse.
    // impact = min(buffer, weight) * 40.0f * phase_multiplier(1.0)
    // We set buffer = 0.1 / 40.0 = 0.0025f, weight = 1.0f.
    for i in 1..=512 {
        graph.connect(i, 0, 1.0, branch_id, false, false);
        let s_idx = graph.synapses.len() - 1;
        graph.synapses[s_idx].neurotransmitter_buffer = 0.1 / 40.0;
        graph.synapses[s_idx].ticks_until_delivery = 1; 
    }

    graph.tick(1, env);

    let target_branch_v = (graph.neurons[0].branches[1].voltage as f32 / 100.0) - 100.0;
    
    println!("Target Branch 1 Voltage: {}", target_branch_v);
    assert!((target_branch_v - 51.2).abs() < 1.5, "Atomic Hammer Failed! Expected ~51.2, got {}", target_branch_v);
}

#[test]
fn test_sequence_recall() {
    println!("--- EXECUTING TEST 66: SEQUENCE RECALL (PREDICTIVE CHAINS) ---");
    let mut graph = NetworkGraph::new();
    
    // A -> B -> C
    graph.add_neuron(NeuronType::Pyramidal); // A (0)
    graph.add_neuron(NeuronType::Pyramidal); // B (1)
    graph.add_neuron(NeuronType::Pyramidal); // C (2)
    graph.neurons[1].branches[4].branch_type = 1;
    graph.neurons[2].branches[4].branch_type = 1;

    // Connect via basal dendrites (Branch 1)
    for _ in 0..10 { graph.connect(0, 1, 10.0, 4, false, false); }
    for _ in 0..10 { graph.connect(1, 2, 10.0, 4, false, false); }

    let mut env = Environment {
        global_oxygen_level: 1.0,
        global_glucose_level: 1.0,
        dopamine_multiplier: 5.0, // High dopamine for training
        toxin_levels: 0.0,
        light_level: 1.0,
        stdp_learning_rate: 0.1,
        wakefulness_multiplier: 1.0,
    };

    // Training 5 times
    for cycle in 0..5 {
        let base_tick = cycle * 30;
        for t in 1..=30 {
            let current_tick = base_tick + t;
            
            // Spike A at 1, B at 10, C at 20
            if t == 1 { graph.neurons[0].current_voltage = 100.0; }
            if t == 10 { graph.neurons[1].current_voltage = 100.0; }
            if t == 20 { graph.neurons[2].current_voltage = 100.0; }

            graph.tick(current_tick, env);

        
        println!("Tick {}: A.last_spike = {}, B.last_spike = {}, B.predictive_ticks = {}, B.branch4_v = {}", current_tick, graph.neurons[0].last_spike_tick, graph.neurons[1].last_spike_tick, graph.neurons[1].predictive_ticks, ((graph.neurons[1].branches[4].voltage as f32) / 100.0) - 100.0);


        }
    }

    // Testing
    env.dopamine_multiplier = 1.0;
    env.stdp_learning_rate = 0.0;

    // Reset predictive states, ATP, Calcium, and refractory
    for n in &mut graph.neurons { 
        n.predictive_ticks = 0; 
        n.current_voltage = n.resting_voltage; 
        n.last_spike_tick = 0; 
        n.energy_atp = n.max_atp;
        n.calcium_level = 0.0;
        n.refractory_time_left = 0.0;
        for b in &mut n.branches {
            b.refractory_timer = 0;
            b.voltage = 10000; // 0.0f
        }
    }

    println!("Weight A->B: {}", graph.synapses[0].weight);
    println!("Weight B->C: {}", graph.synapses[1].weight);
    let test_base_tick = 200;
    for t in 1..=30 {
        let current_tick = test_base_tick + t;
        
        // Spike A at test_base_tick + 1
        if t == 1 { graph.neurons[0].current_voltage = 100.0; }

        graph.tick(current_tick, env);

        
        println!("Tick {}: A.last_spike = {}, B.last_spike = {}, B.predictive_ticks = {}, B.branch4_v = {}", current_tick, graph.neurons[0].last_spike_tick, graph.neurons[1].last_spike_tick, graph.neurons[1].predictive_ticks, ((graph.neurons[1].branches[4].voltage as f32) / 100.0) - 100.0);



        // B should be predictive at A_spike + delay (10 ticks)
        println!("Tick 11: B.predictive_ticks = {}, B.voltage = {}, B.branch4_v = {}", graph.neurons[1].predictive_ticks, graph.neurons[1].current_voltage, ((graph.neurons[1].branches[4].voltage as f32) / 100.0) - 100.0);
        println!("Tick {}: B.predictive_ticks = {}, B.voltage = {}, B.branch4_v = {}", current_tick, graph.neurons[1].predictive_ticks, graph.neurons[1].current_voltage, ((graph.neurons[1].branches[4].voltage as f32) / 100.0) - 100.0);
        if t == 11 {
            assert!(graph.neurons[1].predictive_ticks > 0, "Neuron B did not enter predictive state!");
        }
        
        // C should be predictive at B_spike + delay (20 ticks)? 
        // Wait, if B is just predictive, it won't actually spike unless stimulated.
        // If B doesn't spike, C won't get a signal! 
        // The prompt says: "At tick 100 spike only neuron A. Assert: At tick 110 neuron B should have predictive_ticks > 0. At tick 120 neuron C should have predictive_ticks > 0."
        
        // Let's actually execute the test as described:
        
        // Wait, "Chain of neurons: A -> B -> C... At tick 100 spike only A... At tick 120 neuron C should have predictive_ticks > 0". 
        // We need synapses to form first! We haven't created any synapses.
        
        // But prompt says "Connections via basal dendrites (Branch 1-7)."it's predictive, or maybe the test implies A -> C connection somehow? No, A -> B -> C.
        // If B only gets predictive, it doesn't fire. Thus C receives no signal.
        // Wait, "Chain of neurons: A -> B -> C... At tick 100 spike only A... At tick 120 neuron C should have predictive_ticks > 0". 
        // This means B MUST FIRE. But B is only connected via Basal dendrite. Basal dendrite causes Predictive state, not spike!
        // To make B fire, A would need to connect to B's PROXIMAL dendrite (Branch 0). 
        // But prompt says "Connections via basal dendrites (Branch 1-7)."
        // Is there another mechanism? Spontaneous thermal noise might trigger B if it's predictive (+15mV offset).
        // Let's just follow the prompt exactly and see. 
        if t == 21 {
            assert!(graph.neurons[2].predictive_ticks > 0, "Neuron C did not enter predictive state! (Chain broken)");
        }
    }
    println!("✅ TEST 66 PASS: Predictive Sequence Recall successful.");
}

#[test]
fn test_seizure() {
    println!("--- EXECUTING TEST 67: THE SEIZURE TEST (E/I BALANCE) ---");
    let mut graph = NetworkGraph::new();
    let mut rng = rand::rng();
    
    for i in 0..1000 {
        let n_type = if i < 100 { NeuronType::Interneuron } else { NeuronType::Pyramidal };
        graph.add_neuron(n_type);
    }
    
    // Connect randomly (10 synapses per neuron)
    for i in 0..1000 {
        for _ in 0..10 {
            let target = rand::RngExt::random_range(&mut rng, 0..1000);
            if target == i { continue; }
            let is_inhib = i < 100;
            let weight = if is_inhib { -5.0 } else { 2.0 };
            graph.connect(i, target, weight, 0, is_inhib, false);
        }
    }
    
    let env = Environment {
        global_oxygen_level: 1.0,
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0,
        toxin_levels: 0.0,
        light_level: 1.0,
        stdp_learning_rate: 0.0,
        wakefulness_multiplier: 1.0,
    };
    
    // Induce seizure
    for tick in 1..=5 {
        for i in 0..500 {
            graph.neurons[i].current_voltage = 100.0;
        }
        graph.tick(tick, env);
    }
    
    // Wait for 500 ticks for self-regulation
    for tick in 6..=505 {
        graph.tick(tick, env);
        if tick % 10 == 0 {
            graph.run_astrocyte_service(&env);
        }
    }
    
    let mut active_neurons = 0;
    for n in &graph.neurons {
        if n.last_spike_tick > 500 {
            active_neurons += 1;
        }
    }
    
    println!("Active neurons after seizure: {} / 1000", active_neurons);
    assert!((active_neurons as f32 / 1000.0) < 0.06, "Seizure did not subside! E/I balance broken. Active: {}", active_neurons);
    println!("✅ TEST 67 PASS: Network successfully regulated seizure activity.");
}

#[test]
fn test_signal_race_condition() {
    println!("--- EXECUTING TEST 68: SIGNAL RACE CONDITION (AXON DELAY JITTER) ---");
    let mut graph = NetworkGraph::new();
    
    graph.add_neuron(NeuronType::Pyramidal); // Target T (0)
    graph.add_neuron(NeuronType::Pyramidal); // S1 (1)
    graph.add_neuron(NeuronType::Pyramidal); // S2 (2)
    
    // S1: myelinated (delay 1 tick)
    graph.connect(1, 0, 10.0, 2, false, true); 
    // S2: unmyelinated (delay 4 ticks)
    graph.connect(2, 0, 10.0, 2, false, false);
    
    let env = Environment {
        global_oxygen_level: 1.0,
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0,
        toxin_levels: 0.0,
        light_level: 1.0,
        stdp_learning_rate: 0.0,
        wakefulness_multiplier: 1.0,
    };
    
    for tick in 1..=14 {
        if tick == 10 {
            // S2 (delay 4) spikes
            graph.neurons[2].current_voltage = 100.0;
        }
        if tick == 13 {
            // S1 (delay 1) spikes
            graph.neurons[1].current_voltage = 100.0;
        }
        graph.tick(tick, env);
    }
    
    let target_branch_v = (graph.neurons[0].branches[2].voltage as f32 / 100.0) - 100.0;
    println!("Target Branch 2 Voltage at tick 14: {}", target_branch_v);
    
    // S2 spikes at 10, delay 4 -> delivers at 14.
    // S1 spikes at 13, delay 1 -> delivers at 14.
    // So both hit at tick 14.
    assert!(target_branch_v > 0.0, "Signals did not arrive simultaneously or failed to sum! V = {}", target_branch_v);
    println!("✅ TEST 68 PASS: Signal race condition handled perfectly.");
}

#[test]
fn test_metabolic_darwinism() {
    println!("--- EXECUTING TEST 69: METABOLIC DARWINISM (TOPOLOGY STABILITY) ---");
    let mut graph = NetworkGraph::new();
    let mut rng = rand::rng();
    
    for _ in 0..100 {
        graph.add_neuron(NeuronType::Pyramidal);
    }
    
    // Connect randomly
    for i in 0..100 {
        for _ in 0..5 {
            let target = rand::RngExt::random_range(&mut rng, 0..100);
            if target == i { continue; }
            graph.connect(i, target, 2.0, 0, false, false);
        }
    }
    
    let initial_synapses = graph.synapses.len();
    
    let env = Environment {
        global_oxygen_level: 1.0,
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0,
        toxin_levels: 0.0,
        light_level: 1.0,
        stdp_learning_rate: 0.1,
        wakefulness_multiplier: 1.0,
    };
    
    for tick in 1..=2000 {
        graph.tick(tick, env);
        if tick % 10 == 0 {
            graph.run_astrocyte_service(&env);
        }
        if tick % 100 == 0 {
            graph.run_microglia_service(tick);
            graph.run_neurogenesis_service(tick);
        }
    }
    
    let final_synapses = graph.synapses.len();
    println!("Initial Synapses: {}, Final Synapses: {}", initial_synapses, final_synapses);
    
    let diff = (final_synapses as f32 - initial_synapses as f32).abs() / initial_synapses as f32;
    assert!(final_synapses > 0, "All synapses died!");
    assert!(diff < 0.5, "Synapse count is highly unstable! Diff: {}%", diff * 100.0);
    println!("✅ TEST 69 PASS: Network topology remained stable under neurogenesis/pruning.");
}

#[test]
fn test_inhibitory_stdp() {
    println!("--- EXECUTING TEST 70: INHIBITORY STDP ---");
    let mut graph = NetworkGraph::new();
    
    graph.add_neuron(NeuronType::Pyramidal); // Target P (0)
    graph.add_neuron(NeuronType::Interneuron); // Source I (1)
    
    graph.connect(1, 0, -2.0, 0, true, false); 
    
    let initial_weight = graph.synapses[0].weight;
    
    let env = Environment {
        global_oxygen_level: 1.0,
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0,
        toxin_levels: 0.0,
        light_level: 1.0,
        stdp_learning_rate: 0.1,
        wakefulness_multiplier: 1.0,
    };
    
    for tick in 1..=100 {
        graph.neurons[0].current_voltage = 100.0; // P is hyperactive
        if tick % 5 == 0 {
            graph.neurons[1].current_voltage = 100.0; // I tries to calm it down
        }
        graph.tick(tick, env);
    }
    
    let final_weight = graph.synapses[0].weight;
    println!("Inhibitory Initial Weight: {}, Final Weight: {}", initial_weight, final_weight);
    
    assert!(final_weight != initial_weight, "Inhibitory STDP did not adapt weight! Initial: {}, Final: {}", initial_weight, final_weight);
    println!("✅ TEST 70 PASS: Inhibitory STDP is responsive.");
}
