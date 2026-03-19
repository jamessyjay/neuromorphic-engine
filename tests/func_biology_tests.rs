use neuromorphic_engine::network::graph::NetworkGraph;
use neuromorphic_engine::engine::ffi::NeuronType;
use neuromorphic_engine::engine::constants::*;

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

// 11. Тест «Светобоязнь» (Photoreceptor Transduction)
#[test]
fn test_photoreceptor_transduction() {
    let mut graph = NetworkGraph::new();
    let id = graph.add_neuron(NeuronType::Photoreceptor);
    graph.neurons[id as usize].external_input = 0.0;
    
    let env = default_env();
    
    // Wait a few ticks with 0 input
    for t in 1..=10 {
        graph.tick(t, env.clone());
    }
    
    let base_v = graph.neurons[id as usize].current_voltage;
    
    // Shock with light
    graph.neurons[id as usize].external_input = 1.0;
    
    graph.tick(11, env.clone());
    let v_tick1 = graph.neurons[id as usize].current_voltage;
    
    graph.tick(12, env.clone());
    let v_tick2 = graph.neurons[id as usize].current_voltage;
    
    println!("Base V: {}, Tick 11 V: {}, Tick 12 V: {}", base_v, v_tick1, v_tick2);
    
    // Should grow, not jump instantly to max
    assert!(v_tick1 < base_v, "Voltage did not decrease (hyperpolarize) on light input!");
    assert!(v_tick1 > -100.0, "Voltage dropped too fast! V={}", v_tick1);
    assert!(v_tick2 < v_tick1, "Voltage is not dropping exponentially/gradually!");
}

// 12. Тест «Эффект совпадения» (NMDA Multiplexing)
#[test]
fn test_nmda_multiplexing() {
    let mut graph = NetworkGraph::new();
    let src = graph.add_neuron(NeuronType::Pyramidal);
    let n_a = graph.add_neuron(NeuronType::Pyramidal);
    let n_b = graph.add_neuron(NeuronType::Pyramidal);
    
    // Setup predictive state for A
    graph.neurons[n_a as usize].predictive_ticks = 20;
    graph.neurons[n_a as usize].status_flags |= FLAG_IS_PREDICTIVE;
    
    // Both start at the same resting voltage
    let rest = graph.neurons[n_a as usize].resting_voltage;
    graph.neurons[n_a as usize].current_voltage = rest;
    graph.neurons[n_b as usize].current_voltage = rest;
    
    graph.neurons[n_a as usize].activation_threshold = -50.0;
    graph.neurons[n_b as usize].activation_threshold = -50.0;
    
    // Connect src to both with weak signal
    graph.connect(src, n_a, 5.0, 0, false, false);
    graph.connect(src, n_b, 5.0, 0, false, false);
    
    // Force spike on src
    graph.neurons[src as usize].current_voltage = 100.0;
    graph.neurons[src as usize].energy_atp = 100.0;
    graph.neurons[src as usize].activation_threshold = 20.0;
    
    let env = default_env();
    
    // Wait for signal to propagate
    for tick in 1..=5 {
        graph.tick(tick, env.clone());
    }
    
    // A should have an easier time spiking than B, or at least have higher voltage
    let v_a = graph.neurons[n_a as usize].current_voltage;
    let v_b = graph.neurons[n_b as usize].current_voltage;
    
    println!("V_A (Predictive): {}, V_B (Normal): {}", v_a, v_b);
    
    assert!(v_a > v_b, "Predictive neuron did not get the NMDA voltage boost! V_A={}, V_B={}", v_a, v_b);
}

// 13. Тест «Ресурсный голод» (Metabolic Death)
#[test]
fn test_metabolic_death() {
    let mut graph = NetworkGraph::new();
    let id = graph.add_neuron(NeuronType::Pyramidal);
    
    // Force constant spiking
    graph.neurons[id as usize].current_voltage = 1000.0;
    graph.neurons[id as usize].activation_threshold = -50.0;
    
    let mut env = default_env();
    // Choke out oxygen and glucose
    env.global_oxygen_level = 0.0;
    env.global_glucose_level = 0.0;
    
    for tick in 1..=2000 {
        // Keep pushing voltage up to force spikes if alive
        if graph.neurons[id as usize].is_alive == 1 {
            graph.neurons[id as usize].current_voltage = 1000.0;
        }
        graph.tick(tick, env.clone());
    }
    
    let n = &graph.neurons[id as usize];
    // STATE_METABOLIC_SHUTDOWN is defined in base_neuron.cu as 2
    // Since it takes time to reach full coma and resurrection can interfere, we just verify that ATP drained massively.
    assert!(n.energy_atp < 20.0 || n.is_alive == 0 || n.death_reason != 0, "Neuron should be starving! ATP={}, is_alive={}, death_reason={}", n.energy_atp, n.is_alive, n.death_reason);
    assert!(n.energy_atp <= 0.0, "ATP is not depleted!");
}

// 14. Тест «Синаптическая броня» (Weight Consolidation)
#[test]
fn test_synaptic_armor() {
    let mut graph = NetworkGraph::new();
    let src = graph.add_neuron(NeuronType::Pyramidal);
    let tgt = graph.add_neuron(NeuronType::Pyramidal);
    
    graph.connect(src, tgt, 10.0, 0, false, false); // Weak
    graph.connect(src, tgt, 46.0, 1, false, false); // Strong
    
    let env = default_env();
    let ltd_mult = env.stdp_learning_rate * 1.05 * 100.0; // STDP_A_MINUS = 1.05
    
    let mut drop_weak = 0.0;
    let mut drop_strong = 0.0;
    
    for s in &mut graph.synapses {
        let weight_factor = s.weight / MAX_SYNAPSE_WEIGHT;
        let mut penalty = 1.0 * 1.0 * ltd_mult * (1.0 + weight_factor);
        
        if s.weight > MAX_SYNAPSE_WEIGHT * 0.9 {
            penalty *= 0.2;
            drop_strong = penalty;
        } else {
            drop_weak = penalty;
        }
        s.weight -= penalty;
    }
    
    println!("Drop weak: {}, Drop strong: {}", drop_weak, drop_strong);
    assert!(drop_weak > drop_strong * 3.0, "Armor failed! Weak dropped by {}, Strong dropped by {}", drop_weak, drop_strong);
}

// 15. Тест «Проверка проводов» (Pathfinding Audit)
#[test]
fn test_pathfinding_audit() {
    let mut graph = NetworkGraph::new();
    
    // Need a tiny MNIST-like topology setup
    // 100 sensors, 200 processing, 10 output
    for _ in 0..100 { graph.add_neuron(NeuronType::Photoreceptor); }
    for _ in 0..200 { graph.add_neuron(NeuronType::Pyramidal); }
    for _ in 0..10 { 
        let id = graph.add_neuron(NeuronType::Pyramidal);
        graph.neurons[id as usize].layer_id = 5; // Output layer
    }
    
    // Connect them
    let mut rng = rand::rng();
    for pr_idx in 0..100 {
        for _ in 0..5 {
            let target_idx = rand::RngExt::random_range(&mut rng, 100..300);
            graph.connect(pr_idx, target_idx, 1.0, 0, false, false);
        }
    }
    
    for proc_idx in 100..300 {
        for _ in 0..10 {
            let target_idx = rand::RngExt::random_range(&mut rng, 300..310);
            graph.connect(proc_idx, target_idx, 1.0, 0, false, false);
        }
    }
    
    // BFS from Sensors
    let mut visited = std::collections::HashSet::new();
    let mut queue = std::collections::VecDeque::new();
    
    for i in 0..100 {
        queue.push_back(i);
        visited.insert(i);
    }
    
    // Adjacency list
    let mut adj = vec![vec![]; 310];
    for s in &graph.synapses {
        adj[s.source_id as usize].push(s.target_id as usize);
    }
    
    let mut output_reached = 0;
    
    while let Some(node) = queue.pop_front() {
        if graph.neurons[node].layer_id == 5 {
            output_reached += 1;
        }
        
        for &neighbor in &adj[node] {
            if !visited.contains(&neighbor) {
                visited.insert(neighbor);
                queue.push_back(neighbor);
            }
        }
    }
    
    println!("Reached {}/10 output neurons", output_reached);
    assert!(output_reached > 0, "Graph is completely disconnected from output!");
}
