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
    graph.neurons[1].active_branches_mask |= 1 << 4;
    graph.neurons[1].branches[4].branch_type = 1;
    graph.neurons[2].active_branches_mask |= 1 << 4;
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
        // Spike B at test_base_tick + 11 (after A's predictive signal arrives)
        if t == 11 { graph.neurons[1].current_voltage = 100.0; }

        graph.tick(current_tick, env.clone());

        if t == 11 {
            assert!(graph.neurons[1].predictive_ticks > 0, "Neuron B did not enter predictive state!");
        }
        
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
            // Balance the random E/I connectivity by strengthening inhibition slightly
            let weight = if is_inhib { -15.0 } else { 2.0 };
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
    
    // Inhibitory synapses use positive weights, but have the is_inhibitory flag set
    graph.connect(1, 0, 2.0, 0, true, false); 
    
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

#[test]
fn test_reverberating_circuit() {
    println!("--- EXECUTING TEST 71: REVERBERATING CIRCUIT (AXONAL DELAYS) ---");
    let mut graph = NetworkGraph::new();
    
    // Create a loop: A -> B -> C -> A
    let a = graph.add_neuron(NeuronType::Pyramidal); // 0
    let b = graph.add_neuron(NeuronType::Pyramidal); // 1
    let c = graph.add_neuron(NeuronType::Pyramidal); // 2

    // Set axonal delays specifically to create a slow loop
    // Also reset random properties to ensure reliable looping
    for i in 0..3 {
        let n = &mut graph.neurons[i as usize];
        n.axonal_delay = 5;
        n.activation_threshold = -50.0;
        n.resting_voltage = -70.0;
        n.current_voltage = -70.0;
        n.energy_atp = 1000.0;
        n.max_atp = 1000.0;
        n.vesicles_pool = 1000.0; // High vesicles so they don't fatigue
        n.branches[0].branch_type = 1; // Basal dendrite to trigger spike directly
    }

    // Connect them strongly
    graph.connect(a, b, 50.0, 0, false, false);
    graph.connect(b, c, 50.0, 0, false, false);
    graph.connect(c, a, 50.0, 0, false, false);

    let env = Environment {
        global_oxygen_level: 1.0,
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0,
        toxin_levels: 0.0,
        light_level: 1.0,
        stdp_learning_rate: 0.0,
        wakefulness_multiplier: 1.0,
    };

    // Give them a pre-loaded buffer so they don't fail transmission
    for s in &mut graph.synapses {
        s.neurotransmitter_buffer = 100.0;
    }

    // Inject one initial spike into A
    graph.neurons[a as usize].current_voltage = 200.0; // Over-spike to guarantee transmission
    
    let mut spikes_recorded = 0;
    
    for tick in 1..=50 {
        // Feed them to prevent fatigue
        graph.neurons[a as usize].energy_atp = 100.0;
        graph.neurons[b as usize].energy_atp = 100.0;
        graph.neurons[c as usize].energy_atp = 100.0;
        
        graph.tick(tick, env);
        
        if graph.neurons[a as usize].last_spike_tick == tick { spikes_recorded += 1; }
        if graph.neurons[b as usize].last_spike_tick == tick { spikes_recorded += 1; }
        if graph.neurons[c as usize].last_spike_tick == tick { spikes_recorded += 1; }
        
        // Debug output to trace the loop
        if tick % 5 == 0 {
            let va = graph.neurons[a as usize].current_voltage;
            let vb = graph.neurons[b as usize].current_voltage;
            let vc = graph.neurons[c as usize].current_voltage;
            println!("Tick {:2}: A={:5.1} | B={:5.1} | C={:5.1}", tick, va, vb, vc);
        }
    }
    
    // In 50 ticks with delays of 5 each (total loop 15 ticks), 
    // we expect at least 3 loops (9 total spikes)
    println!("Total spikes recorded in 50 ticks: {}", spikes_recorded);
    assert!(spikes_recorded >= 9, "Spike died out before completing reverberating loops! Recorded: {}", spikes_recorded);
    
    // Check if there was recent activity near the end of the simulation
    let latest_spike = graph.neurons[a as usize].last_spike_tick
        .max(graph.neurons[b as usize].last_spike_tick)
        .max(graph.neurons[c as usize].last_spike_tick);
        
    assert!(latest_spike > 40, "Signal did not survive until the end of the 50 tick window!");
    
    println!("✅ TEST 71 PASS: Reverberating circuit successfully maintained signal.");
}
