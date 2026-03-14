use neuromorphic_engine::modes::setup_benchmark;
use neuromorphic_engine::ffi::NeuronType;

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

    let expected_sensors = (num_neurons as f32 * 0.04) as usize;
    let expected_motors = (num_neurons as f32 * 0.04) as usize;
    let expected_pacemakers = (num_neurons as f32 * 0.02) as usize;
    let expected_interneurons = (num_neurons as f32 * 0.15) as usize;
    let expected_pyramidal = num_neurons - expected_sensors - expected_motors - expected_pacemakers - expected_interneurons;

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
        let has_inhibitory_flag = (syn.flags & 0x08) != 0;

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
    assert!(checked_inhibitory > 0, "No inhibitory synapses were generated!");
    assert!(checked_excitatory > 0, "No excitatory synapses were generated!");
}
