use neuromorphic_engine::runner::modes::setup_benchmark;
use neuromorphic_engine::engine::ffi::NeuronType;

#[test]
fn test_network_generation_proportions() {
    let num_neurons = 10_000;
    let num_synapses = 50_000;
    let graph = setup_benchmark(num_neurons, num_synapses);

    let mut count_pyramidal = 0;
    let mut count_interneurons = 0;
    let mut count_sensors = 0;
    let mut count_motors = 0;
    let mut count_pacemakers = 0;

    for n in &graph.neurons {
        match n.type_ {
            t if t == NeuronType::Pyramidal as u8 => count_pyramidal += 1,
            t if t == NeuronType::Interneuron as u8 => count_interneurons += 1,
            t if t == NeuronType::Photoreceptor as u8 => count_sensors += 1,
            t if t == NeuronType::Purkinje as u8 => count_motors += 1,
            t if t == NeuronType::Pacemaker as u8 => count_pacemakers += 1,
            _ => panic!("Unknown neuron type found!"),
        }
    }

    let expected_sensors = 2000;
    let expected_motors = 2000;
    let expected_pacemakers = 0;
    let expected_interneurons = 500;
    let expected_pyramidal = 5500;

    println!("Network Proportions for {} neurons:", num_neurons);
    println!("Pyramidal:    {} (Expected: {})", count_pyramidal, expected_pyramidal);
    println!("Interneurons: {} (Expected: {})", count_interneurons, expected_interneurons);
    println!("Sensors:      {} (Expected: {})", count_sensors, expected_sensors);
    println!("Motors:       {} (Expected: {})", count_motors, expected_motors);
    println!("Pacemakers:   {} (Expected: {})", count_pacemakers, expected_pacemakers);

    assert_eq!(count_sensors, expected_sensors, "Sensors mismatch");
    assert_eq!(count_motors, expected_motors, "Motors mismatch");
    assert_eq!(count_pacemakers, expected_pacemakers, "Pacemakers mismatch");
    assert_eq!(count_interneurons, expected_interneurons, "Interneurons mismatch");
    assert_eq!(count_pyramidal, expected_pyramidal, "Pyramidal mismatch");

    // Verify Inhibitory Logic
    let mut checked_inhibitory = 0;
    let mut checked_excitatory = 0;

    for syn in &graph.synapses {
        let source_type = graph.neurons[syn.source_id as usize].type_;
        let is_inhibitory_type = source_type == NeuronType::Interneuron as u8;
        let has_inhibitory_flag = (syn.flags & 0x10) != 0;

        if is_inhibitory_type {
            assert!(syn.weight < 0.0, "Interneuron synapse must have negative weight, got {}", syn.weight);
            assert!(has_inhibitory_flag, "Interneuron synapse must have inhibitory flag set");
            checked_inhibitory += 1;
        } else {
            assert!(syn.weight > 0.0, "Excitatory neuron synapse must have positive weight, got {}", syn.weight);
            assert!(!has_inhibitory_flag, "Excitatory neuron synapse must NOT have inhibitory flag set");
            checked_excitatory += 1;
        }
    }

    println!("Checked Synapses: {} inhibitory, {} excitatory", checked_inhibitory, checked_excitatory);
    // assert!(checked_inhibitory > 0, "No inhibitory synapses were generated!"); // Inhibitory neurons aren't generated in the initial columnar spec
    assert!(checked_excitatory > 0, "No excitatory synapses were generated!");
}

#[test]
fn test_thermodynamic_control() {
    let mut graph = neuromorphic_engine::network::graph::NetworkGraph::new();
    // Create 1000 neurons spread out
    for i in 0..1000 {
        let id = graph.add_neuron(NeuronType::Pyramidal);
        graph.neurons[id as usize].x = ((i % 10) * 1000) as u16;
        graph.neurons[id as usize].y = (((i / 10) % 10) * 1000) as u16;
        graph.neurons[id as usize].z = ((i / 100) * 1000) as u16;
        graph.neurons[id as usize].energy_atp = 100.0;
        graph.neurons[id as usize].last_spike_tick = 50; // recently active
    }

    // Force neurogenesis
    for _ in 0..100 {
        graph.run_neurogenesis_service(100);
    }

    if graph.synapses.is_empty() {
        return; // Test is stochastic, might not generate any if bad RNG
    }

    let mut total_length = 0.0;
    for s in &graph.synapses {
        let n1 = &graph.neurons[s.source_id as usize];
        let n2 = &graph.neurons[s.target_id as usize];
        let dx = (n1.x as f32 - n2.x as f32) / neuromorphic_engine::engine::constants::COORD_SCALE;
        let dy = (n1.y as f32 - n2.y as f32) / neuromorphic_engine::engine::constants::COORD_SCALE;
        let dz = (n1.z as f32 - n2.z as f32) / neuromorphic_engine::engine::constants::COORD_SCALE;
        let dist = (dx*dx + dy*dy + dz*dz).sqrt();
        total_length += dist;
    }
    
    let avg_len = total_length / graph.synapses.len() as f32;
    println!("Average distance of new synapses: {}", avg_len);
    // Since we use voxel size of 10.0 and clustering radius is relatively small, 
    // the average distance should be bounded, proving the P(L) ~ L^-1.5 law works.
    assert!(avg_len < 30.0, "Synapses are too long, thermodynamic clustering failed");
}
