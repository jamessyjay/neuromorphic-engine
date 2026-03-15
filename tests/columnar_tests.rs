use neuromorphic_engine::network::graph::NetworkGraph;
use neuromorphic_engine::engine::ffi::{Environment, NeuronType, BaseNeuron};
use neuromorphic_engine::engine::constants;

#[test]
fn test_columnar_assignment() {
    let mut graph = NetworkGraph::new();
    
    // We can simulate network generation logic here, or test that `column_id` can be set correctly
    for i in 0..100 {
        graph.add_neuron(NeuronType::Pyramidal);
        let id = graph.neurons.len() - 1;
        graph.neurons[id].column_id = (i / 10) as u16;
        graph.neurons[id].minicolumn_id = (i % 5) as u8;
        graph.neurons[id].layer_id = 4;
        graph.neurons[id].z = 400; // Let's say layer 4 is at Z=400
    }
    
    // Verify properties
    let layer_4_neurons: Vec<&BaseNeuron> = graph.neurons.iter().filter(|n| n.layer_id == 4).collect();
    assert_eq!(layer_4_neurons.len(), 100);
    
    // Same column has same column_id
    assert_eq!(graph.neurons[0].column_id, 0);
    assert_eq!(graph.neurons[9].column_id, 0);
    assert_eq!(graph.neurons[10].column_id, 1);
}

#[test]
fn test_minicolumn_wta_inhibition() {
    let mut graph = NetworkGraph::new();
    
    // Create a minicolumn with 10 neurons
    for _ in 0..10 {
        graph.add_neuron(NeuronType::Pyramidal);
        let id = graph.neurons.len() - 1;
        graph.neurons[id].column_id = 1;
        graph.neurons[id].minicolumn_id = 1;
    }
    
    let mut env = Environment {
        global_oxygen_level: 1.0,
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0,
        toxin_levels: 0.0,
        light_level: 0.0,
        stdp_learning_rate: 0.1,
        wakefulness_multiplier: 1.0,
    };
    
    // Force spike on Neuron 1
    graph.force_sync_gpu();
    graph.neurons[1].current_voltage = 50.0;
    
    // Partially excite Neuron 2, but less than Neuron 1
    graph.neurons[2].current_voltage = 40.0;
    
    // Run tick (WTA lateral inhibition applies BEFORE physics)
    graph.apply_wta_inhibition();
    println!("Before tick, Loser: v={}", graph.neurons[2].current_voltage);
    graph.tick(1, env);
    
    // Neuron 1 should have won and spiked, Neuron 2 should have been inhibited
    assert!(graph.neurons[1].current_voltage > 10.0 || graph.neurons[1].last_spike_tick == 1, "Winner should spike");
    println!("Loser voltage: {}, Threshold: {}, Status Flags: {}, Is inhibited flag: {}", graph.neurons[2].current_voltage, graph.neurons[2].activation_threshold, graph.neurons[2].status_flags, graph.neurons[2].status_flags & 1);
    assert!(graph.neurons[2].current_voltage < 30.0, "Loser should be hyperpolarized. Voltage was: {}", graph.neurons[2].current_voltage);
    assert_eq!((graph.neurons[2].status_flags & constants::FLAG_IS_INHIBITED) > 0, true, "Loser must have FLAG_IS_INHIBITED");
}
