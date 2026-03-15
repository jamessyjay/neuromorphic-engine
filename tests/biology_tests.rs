use neuromorphic_engine::engine::snapshot::BrainSnapshot;
use neuromorphic_engine::network::graph::NetworkGraph;
use neuromorphic_engine::engine::ffi::{Environment, NeuronType};

fn default_env() -> Environment {
    Environment {
        global_oxygen_level: 1.0,
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0,
        toxin_levels: 0.0,
        light_level: 0.0, stdp_learning_rate: 0.1,
            wakefulness_multiplier: 1.0,
    }
}

#[test]
fn test_metabolic_sink() {
    let mut graph = NetworkGraph::new();
    for _ in 0..100 {
        graph.add_neuron(NeuronType::Pyramidal);
        let id = graph.neurons.len() - 1;
        graph.neurons[id].column_id = id as u16;
        graph.neurons[id].minicolumn_id = 1;
    }
    
    let mut env = default_env();
    env.global_oxygen_level = 0.2;
    env.wakefulness_multiplier = 0.0; // Disable recovery so passive drain and spike cost can kill them

    // Grace period protects neurons until tick 5000
    for tick in 5001..=5200 {
        for n in &mut graph.neurons {
            if n.is_alive == 1 {
                n.current_voltage = 100.0; 
            }
        }
        graph.tick(tick, env.clone());
    }
    
    let alive_count = graph.neurons.iter().filter(|n| n.is_alive == 1).count();
    assert_eq!(alive_count, 0, "All neurons should be dead due to metabolic sink (hypoxia + forced spiking).");
    assert_eq!(graph.neurons[0].death_reason, 1 /* DEATH_ATP_LOW */);
}

#[test]
fn test_atp_recovery_asymptote() {
    let mut graph = NetworkGraph::new();
    let id = graph.add_neuron(NeuronType::Pyramidal);
        let id = graph.neurons.len() - 1;
        graph.neurons[id].column_id = id as u16;
        graph.neurons[id].minicolumn_id = 1;
    let mut env = default_env();
    env.global_oxygen_level = 1.0;
    env.global_glucose_level = 1.0;

    // Drain some ATP first
    graph.neurons[id as usize].energy_atp = 50.0;
    
    let mut recovery_rates = vec![];
    
    for tick in 1..=200 {
        let prev_atp = graph.neurons[id as usize].energy_atp;
        graph.tick(tick, env.clone());
        let current_atp = graph.neurons[id as usize].energy_atp;
        
        let rate = current_atp - prev_atp;
        recovery_rates.push(rate);
    }
    
    assert!(recovery_rates[0] > recovery_rates[150], "Recovery should slow down logarithmically as it approaches max_atp");
}

#[test]
fn test_mitochondrial_failure() {
    let mut graph = NetworkGraph::new();
    for _ in 0..100 {
        graph.add_neuron(NeuronType::Pyramidal);
        let id = graph.neurons.len() - 1;
        graph.neurons[id].column_id = id as u16;
        graph.neurons[id].minicolumn_id = 1;
    }
    
    let mut env = default_env();
    env.global_glucose_level = 0.05; // Critical glucose
    env.global_oxygen_level = 0.2; // Hypoxia required for death
    env.wakefulness_multiplier = 0.0; // Disable recovery so passive drain can kill them

    for tick in 5001..=5500 {
        // Minimal activity to burn ATP slowly
        if tick % 10 == 0 {
            for n in &mut graph.neurons {
                if n.is_alive == 1 {
                    n.current_voltage = 100.0; 
                }
            }
        }
        graph.tick(tick, env.clone());
    }
    
    let alive_count = graph.neurons.iter().filter(|n| n.is_alive == 1).count();
    assert_eq!(alive_count, 0, "Neurons should die from starvation with low glucose.");
}

#[test]
fn test_metabolic_floor_leak() {
    let mut graph = NetworkGraph::new();
    for _ in 0..100 {
        graph.add_neuron(NeuronType::Pyramidal);
        let id = graph.neurons.len() - 1;
        graph.neurons[id].column_id = id as u16;
        graph.neurons[id].minicolumn_id = 1;
    }
    
    let env = default_env();

    for tick in 1..=1000 {
        graph.tick(tick, env.clone());
    }
    
    let alive_count = graph.neurons.iter().filter(|n| n.is_alive == 1).count();
    assert_eq!(alive_count, 100, "Neurons should survive on idle due to glucose processing and recovery compensating for passive drain.");
}

#[test]
fn test_oxygen_deprivation() {
    let mut graph = NetworkGraph::new();
    for _ in 0..100 {
        graph.add_neuron(NeuronType::Pyramidal);
        let id = graph.neurons.len() - 1;
        graph.neurons[id].column_id = id as u16;
        graph.neurons[id].minicolumn_id = 1;
    }
    
    let mut env = default_env();
    env.global_oxygen_level = 0.0; // Total hypoxia
    env.wakefulness_multiplier = 0.0; // Disable recovery so passive drain can kill them

    for n in &mut graph.neurons {
        n.energy_atp = 0.1; // Set very low ATP so they die almost instantly
    }

    for tick in 5001..=5050 {
        graph.tick(tick, env.clone());
    }
    
    let alive_count = graph.neurons.iter().filter(|n| n.is_alive == 1).count();
    println!("Alive after hypoxia: {}", alive_count);
    assert_eq!(alive_count, 0, "All neurons should die rapidly due to oxygen deprivation passive drain.");
}

#[test]
fn test_calcium_overdose() {
    let mut graph = NetworkGraph::new();
    let id = graph.add_neuron(NeuronType::Pyramidal);
        let id = graph.neurons.len() - 1;
        graph.neurons[id].column_id = id as u16;
        graph.neurons[id].minicolumn_id = 1;
    let env = default_env();
    
    for tick in 5001..=5500 {
        // Force spike every tick, bypassing refractory by artificially resetting it
        if graph.neurons[id as usize].is_alive == 1 {
            graph.neurons[id as usize].current_voltage = 100.0; 
            graph.neurons[id as usize].refractory_time_left = 0.0;
            // Also supply ATP so it doesn't just sleep
            graph.neurons[id as usize].energy_atp = 100.0;
        }
        graph.tick(tick, env.clone());
    }
    
    println!("Calcium after forced spikes: {}, Alive: {}", graph.neurons[id as usize].calcium_level, graph.neurons[id as usize].is_alive);
    assert_eq!(graph.neurons[id as usize].is_alive, 0, "Neuron should die from excitotoxicity (calcium overdose).");
    assert_eq!(graph.neurons[id as usize].death_reason, 2 /* DEATH_CALCIUM_HIGH */);
}

#[test]
fn test_calcium_recovery_rate() {
    let mut graph = NetworkGraph::new();
    let id = graph.add_neuron(NeuronType::Pyramidal);
        let id = graph.neurons.len() - 1;
        graph.neurons[id].column_id = id as u16;
        graph.neurons[id].minicolumn_id = 1;
    let env = default_env();
    
    // Induce a single spike manually
    graph.neurons[id as usize].calcium_level = 50.0;
    
    for tick in 1..=200 {
        graph.tick(tick, env.clone());
    }
    
    assert!(graph.neurons[id as usize].calcium_level < 1.0, "Calcium should have recovered fully after idle period.");
    assert_eq!(graph.neurons[id as usize].is_alive, 1, "Neuron should survive normal calcium load.");
}

#[test]
fn test_l6_coord_persistence() {
    let mut graph = NetworkGraph::new();
    graph.add_neuron(NeuronType::Pyramidal);
        let id = graph.neurons.len() - 1;
        graph.neurons[id].column_id = id as u16;
        graph.neurons[id].minicolumn_id = 1;
    
    // Assign specific ref coords for L6 simulation
    graph.neurons[0].ref_coord_x = 42.5;
    graph.neurons[0].ref_coord_y = 13.37;
    
    // Save to disk
    let test_file = "test_l6_save.bin";
    BrainSnapshot::save_test_snapshot(100, &graph, test_file);
    
    // Load back
    let snapshot = BrainSnapshot::load_from_disk(test_file).expect("Failed to load");
    
    // Verify persistence
    assert_eq!(snapshot.graph.neurons[0].ref_coord_x, 42.5);
    assert_eq!(snapshot.graph.neurons[0].ref_coord_y, 13.37);
    
    // Cleanup
    let _ = std::fs::remove_file(test_file);
}
