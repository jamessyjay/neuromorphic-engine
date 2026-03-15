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
    
    let env = Environment {
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

#[test]
fn test_z_boundaries() {
    let graph = neuromorphic_engine::runner::modes::setup_benchmark(10000, 100);
    
    use neuromorphic_engine::engine::constants::*;

    for n in &graph.neurons {
        match n.layer_id {
            6 => assert!(n.z >= LAYER_6_Z_MIN && n.z <= LAYER_6_Z_MAX, "L6 neuron {} has z {} out of bounds", n.id, n.z),
            5 => assert!(n.z >= LAYER_5_Z_MIN && n.z <= LAYER_5_Z_MAX, "L5 neuron {} has z {} out of bounds", n.id, n.z),
            4 => assert!(n.z >= LAYER_4_Z_MIN && n.z <= LAYER_4_Z_MAX, "L4 neuron {} has z {} out of bounds", n.id, n.z),
            2 => assert!(n.z >= LAYER_2_3_Z_MIN && n.z <= LAYER_2_3_Z_MAX, "L2/3 neuron {} has z {} out of bounds", n.id, n.z),
            1 => assert!(n.z >= 60001, "L1 neuron {} has z {} out of bounds", n.id, n.z), // Note: n.z is u16, so it is inherently <= 65535
            _ => panic!("Unknown layer ID {}", n.layer_id),
        }
    }
}

#[test]
fn test_radial_clustering() {
    let graph = neuromorphic_engine::runner::modes::setup_benchmark(10000, 100);
    
    
    let column_centers_x: Vec<f32> = (0..5).map(|i| (i as f32) * 12000.0 + 10000.0).collect();
    let column_centers_y: Vec<f32> = (0..2).map(|i| (i as f32) * 20000.0 + 10000.0).collect();

    for n in &graph.neurons {
        let center_x = column_centers_x[(n.column_id % 5) as usize];
        let center_y = column_centers_y[(n.column_id / 5) as usize];
        
        let dx = n.x as f32 - center_x;
        let dy = n.y as f32 - center_y;
        
        // Assert that the neuron is within a radius of 5000 from the column center
        assert!(dx.abs() <= 5000.0, "Neuron {} X coordinate {} drifted too far from center {}", n.id, n.x, center_x);
        assert!(dy.abs() <= 5000.0, "Neuron {} Y coordinate {} drifted too far from center {}", n.id, n.y, center_y);
    }
}

#[test]
fn test_minicolumn_distribution() {
    let graph = neuromorphic_engine::runner::modes::setup_benchmark(10000, 100);
    
    use std::collections::HashMap;
    let mut minicol_counts: HashMap<(u16, u8), usize> = HashMap::new();
    
    for n in &graph.neurons {
        *minicol_counts.entry((n.column_id, n.minicolumn_id)).or_insert(0) += 1;
    }
    
    for count in minicol_counts.values() {
        assert_eq!(*count, 20, "Each minicolumn should contain exactly 20 neurons (1000 per column / 50 minicolumns)");
    }
}

#[test]
fn test_connectivity_enforcement() {
    let graph = neuromorphic_engine::runner::modes::setup_benchmark(1000, 10000); // Small dense network
    
    let mut l6_to_l5_count = 0;
    
    for syn in &graph.synapses {
        let n_source = &graph.neurons[syn.source_id as usize];
        let n_target = &graph.neurons[syn.target_id as usize];
        
        if n_source.layer_id == 6 && n_target.layer_id == 5 {
            l6_to_l5_count += 1;
        }
    }
    
    assert_eq!(l6_to_l5_count, 0, "L6 must NOT directly connect to L5! Connectivity ban failed.");
}

#[test]
fn test_sensory_projection_accuracy() {
    let mut graph = neuromorphic_engine::runner::modes::setup_benchmark(10000, 100);
    let mut hub = neuromorphic_engine::network::stimulus::SensoryHub::new();
    hub.build_mapping(&graph);
    
    // 1. Clear frame
    let mut frame = vec![0.0; 1024];
    
    // 2. Light up pixel (0,0) which maps to index 0
    for i in 0..102 { frame[i] = 1.0; } // Light up the entire patch for Column 0
    hub.update_frame(frame);
    
    // 3. Apply to graph (simulates stimulus injection)
    hub.apply_to_graph(&mut graph);
    
    // 4. Verification
    let mut active_col_1 = 0;
    let mut active_col_10 = 0;
    let mut total_l4 = 0;
    let mut total_active_l4 = 0;
    
    for n in &graph.neurons {
        if n.layer_id == 4 {
            total_l4 += 1;
            if n.current_voltage > 100.0 { // Was boosted to STIMULUS_VOLTAGE_BOOST (150.0)
                total_active_l4 += 1;
                if n.column_id == 0 { active_col_1 += 1; }
                if n.column_id == 9 { active_col_10 += 1; }
            }
        }
    }
    
    assert!(active_col_1 > 0, "Column 0 should have received stimulus from pixel 0!");
    assert_eq!(active_col_10, 0, "Column 9 (last column) should NOT receive stimulus from pixel 0!");
    
    let sparsity = total_active_l4 as f32 / total_l4 as f32;
    println!("SDR Sparsity for single pixel activation: {:.2}%", sparsity * 100.0);
    // Since pixel 0 maps only to column 0, the overall sparsity should be very low, definitely < 5-7%
    assert!(sparsity < 0.07, "SDR Sparsity violated! Expected < 7%, got {:.2}%", sparsity * 100.0);
}

#[test]
fn test_topology_final_verification() {
    let graph = neuromorphic_engine::runner::modes::setup_benchmark(10000, 0);
    
    // 1. Check for clustering (columns shouldn't be all at the same X point)
    let n0 = &graph.neurons[0];
    let n9000 = &graph.neurons[9000];
    assert!(n0.x != n9000.x, "Error: All columns are in the same X position!");

    // 2. Check layers
    let photoneuron = graph.neurons.iter().find(|n| n.layer_id == 4).unwrap();
    assert_eq!(photoneuron.type_ as u8, 5, "Error: Layer 4 must consist of Photoreceptors!");

    // 3. Check metadata mapping correctly
    assert!(graph.neurons.iter().any(|n| n.column_id == 9), "Error: Column IDs are not assigned correctly!");
}

#[test]
fn test_no_orphan_neurons() {
    // Generate a full network using the new Setup logic
    let graph = neuromorphic_engine::runner::modes::setup_benchmark(10000, 10000);
    
    let mut out_counts = vec![0; graph.neurons.len()];
    for s in &graph.synapses {
        out_counts[s.source_id as usize] += 1;
    }
    
    // Check that every neuron has at least 3 outgoing synapses (except if there are some edge cases, 
    // but the spec dictates out_synapses >= 3 is guaranteed by the new Orphan logic)
    for (i, n) in graph.neurons.iter().enumerate() {
        assert!(out_counts[i] >= 3, "Orphan detected! Neuron {} (Layer {}) has only {} outgoing synapses.", i, n.layer_id, out_counts[i]);
    }
}

#[test]
fn test_wta_suppression() {
    let mut graph = NetworkGraph::new();
    
    // Create a minicolumn with 2 neurons (Pyramidal)
    for _i in 0..2 {
        graph.add_neuron(NeuronType::Pyramidal);
        let id = graph.neurons.len() - 1;
        graph.neurons[id].column_id = 1;
        graph.neurons[id].minicolumn_id = 1;
        graph.neurons[id].layer_id = 2; // L2/3
        graph.neurons[id].is_alive = 1;
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
    
    // Force spike on Neuron A
    graph.force_sync_gpu();
    graph.neurons[0].current_voltage = 50.0; // Above threshold to trigger spike in phase 1
    
    // Run tick 1
    graph.tick(1, env);
    
    // Neuron 0 should have spiked in tick 1
    assert_eq!(graph.neurons[0].last_spike_tick, 1, "Neuron A should have spiked");
    
    // Check Neuron 1 (Neuron B) for FLAG_IS_INHIBITED right after the tick where Neuron A spiked
    assert_eq!((graph.neurons[1].status_flags & constants::FLAG_IS_INHIBITED) > 0, true, "Neuron B must have FLAG_IS_INHIBITED set due to WTA suppression from Neuron A.");
}
