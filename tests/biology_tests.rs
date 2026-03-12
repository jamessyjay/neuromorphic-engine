use neuromorphic_engine::graph::NetworkGraph;
use neuromorphic_engine::ffi::{Environment, NeuronType};

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

#[test]
fn test_metabolic_sink() {
    let mut graph = NetworkGraph::new();
    for _ in 0..100 {
        graph.add_neuron(NeuronType::Pyramidal);
    }
    
    let mut env = default_env();
    env.global_oxygen_level = 0.4;

    for tick in 1..=200 {
        for n in &mut graph.neurons {
            if n.is_alive {
                n.current_voltage = 100.0; 
            }
        }
        graph.tick(tick, env.clone());
    }
    
    let alive_count = graph.neurons.iter().filter(|n| n.is_alive).count();
    assert_eq!(alive_count, 0, "All neurons should be dead due to metabolic sink (hypoxia + forced spiking).");
    assert_eq!(graph.neurons[0].death_reason, 1 /* DEATH_ATP_LOW */);
}

#[test]
fn test_atp_recovery_asymptote() {
    let mut graph = NetworkGraph::new();
    let id = graph.add_neuron(NeuronType::Pyramidal);
    let env = default_env();
    
    graph.neurons[id as usize].energy_atp = 0.1;
    
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
    }
    
    let mut env = default_env();
    env.global_glucose_level = 0.05; // Critical glucose
    env.global_oxygen_level = 0.4; // Hypoxia required for death

    for tick in 1..=500 {
        // Minimal activity to burn ATP slowly
        if tick % 10 == 0 {
            for n in &mut graph.neurons {
                if n.is_alive {
                    n.current_voltage = 100.0; 
                }
            }
        }
        graph.tick(tick, env.clone());
    }
    
    let alive_count = graph.neurons.iter().filter(|n| n.is_alive).count();
    assert_eq!(alive_count, 0, "Neurons should die from starvation with low glucose.");
}

#[test]
fn test_metabolic_floor_leak() {
    let mut graph = NetworkGraph::new();
    for _ in 0..100 {
        graph.add_neuron(NeuronType::Pyramidal);
    }
    
    let env = default_env();

    for tick in 1..=1000 {
        graph.tick(tick, env.clone());
    }
    
    let alive_count = graph.neurons.iter().filter(|n| n.is_alive).count();
    assert_eq!(alive_count, 100, "Neurons should survive on idle due to glucose processing and recovery compensating for passive drain.");
}

#[test]
fn test_oxygen_deprivation() {
    let mut graph = NetworkGraph::new();
    for _ in 0..100 {
        graph.add_neuron(NeuronType::Pyramidal);
    }
    
    let mut env = default_env();
    env.global_oxygen_level = 0.0; // Total hypoxia

    for tick in 1..=5000 {
        graph.tick(tick, env.clone());
    }
    
    let alive_count = graph.neurons.iter().filter(|n| n.is_alive).count();
    println!("Alive after hypoxia: {}", alive_count);
    assert_eq!(alive_count, 0, "All neurons should die rapidly due to oxygen deprivation passive drain.");
}

#[test]
fn test_calcium_overdose() {
    let mut graph = NetworkGraph::new();
    let id = graph.add_neuron(NeuronType::Pyramidal);
    let env = default_env();
    
    for tick in 1..=500 {
        // Force spike every tick, bypassing refractory by artificially resetting it
        if graph.neurons[id as usize].is_alive {
            graph.neurons[id as usize].current_voltage = 100.0; 
            graph.neurons[id as usize].refractory_time_left = 0.0;
            // Also supply ATP so it doesn't just sleep
            graph.neurons[id as usize].energy_atp = 100.0;
        }
        graph.tick(tick, env.clone());
    }
    
    println!("Calcium after forced spikes: {}, Alive: {}", graph.neurons[id as usize].calcium_level, graph.neurons[id as usize].is_alive);
    assert_eq!(graph.neurons[id as usize].is_alive, false, "Neuron should die from excitotoxicity (calcium overdose).");
    assert_eq!(graph.neurons[id as usize].death_reason, 2 /* DEATH_CALCIUM_HIGH */);
}

#[test]
fn test_calcium_recovery_rate() {
    let mut graph = NetworkGraph::new();
    let id = graph.add_neuron(NeuronType::Pyramidal);
    let env = default_env();
    
    // Induce a single spike manually
    graph.neurons[id as usize].calcium_level = 50.0;
    
    for tick in 1..=200 {
        graph.tick(tick, env.clone());
    }
    
    assert!(graph.neurons[id as usize].calcium_level < 1.0, "Calcium should have recovered fully after idle period.");
    assert_eq!(graph.neurons[id as usize].is_alive, true, "Neuron should survive normal calcium load.");
}
