use neuromorphic_engine::network::graph::NetworkGraph;
use neuromorphic_engine::engine::ffi::{Environment, NeuronType};

#[test]
fn test_sdr_activity_limit() {
    let mut graph = NetworkGraph::new();
    
    // Create 1000 neurons, grouped into minicolumns of 10
    for i in 0..1000 {
        graph.add_neuron(NeuronType::Pyramidal);
        let id = graph.neurons.len() - 1;
        graph.neurons[id].column_id = (i / 100) as u16;
        graph.neurons[id].minicolumn_id = ((i % 100) / 10) as u8;
        graph.neurons[id].layer_id = 4;
        
         
    }
    
    graph.force_sync_gpu();
    
    // Blast them all with massive excitation (noise)
    for i in 0..1000 {
        graph.neurons[i].current_voltage = 50.0 + (i % 20) as f32;
    }
    
    let env = Environment {
        global_oxygen_level: 1.0,
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0,
        toxin_levels: 0.0,
        light_level: 0.0,
        stdp_learning_rate: 0.1,
        wakefulness_multiplier: 1.0,
    };
    
    // Process tick (WTA + Physics)
    graph.tick(1, env);
    
    let active_neurons = graph.neurons.iter().filter(|n| n.last_spike_tick == 1).count();
    let sparsity_percent = (active_neurons as f32 / 1000.0) * 100.0;
    
    println!("Sparsity: {:.2}% ({} active out of 1000)", sparsity_percent, active_neurons);
    
    // SDR target: 2-10% sparsity
    assert!(sparsity_percent > 1.0 && sparsity_percent <= 15.0, "Sparsity should be highly regulated via WTA. Got {}%", sparsity_percent);
}
