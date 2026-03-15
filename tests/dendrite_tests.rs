use neuromorphic_engine::engine::ffi::Synapse;
use neuromorphic_engine::network::graph::NetworkGraph;
use neuromorphic_engine::engine::ffi::{Environment, NeuronType, Compartment};

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

// Macro to pack/unpack floats to fixed-point for tests (matching C++ macros)
fn pack_voltage(v: f32) -> u16 {
    let mut shifted = v + 100.0;
    if shifted < 0.0 { shifted = 0.0; }
    if shifted > 65535.0 / 100.0 { shifted = 65535.0 / 100.0; }
    (shifted * 100.0) as u16
}

fn unpack_voltage(v: u16) -> f32 {
    (v as f32 / 100.0) - 100.0
}

#[test]
fn test_spatial_summation_basal() {
    let mut graph = NetworkGraph::new();
    let id = graph.add_neuron(NeuronType::Pyramidal);
    let env = default_env();
    
    // Set up branch 1 as Basal
    graph.neurons[id as usize].branches[1] = Compartment {
        branch_type: 1, // Basal
        refractory_timer: 0,
        voltage: pack_voltage(0.0),
        nmda_threshold: pack_voltage(50.0), // High threshold to avoid NMDA spike
        local_trace: 0,
            _pad: 0,
    };
    
    let initial_voltage = graph.neurons[id as usize].current_voltage;
    
    for tick in 1..=10 {
        // Send a weak signal to branch 1
        let current_branch_v = unpack_voltage(graph.neurons[id as usize].branches[1].voltage);
        graph.neurons[id as usize].branches[1].voltage = pack_voltage(current_branch_v + 10.0);
        
        graph.tick(tick, env.clone());
    }
    
    let final_voltage = graph.neurons[id as usize].current_voltage;
    println!("initial: {}, final: {}", initial_voltage, final_voltage);
    assert!(final_voltage > initial_voltage, "Soma voltage should increase due to spatial summation");
}

#[test]
fn test_nmda_spike_trigger() {
    let mut graph = NetworkGraph::new();
    let id = graph.add_neuron(NeuronType::Pyramidal);
    let env = default_env();
    
    graph.neurons[id as usize].current_voltage = -70.0;
    let initial_voltage = -70.0;

    graph.neurons[id as usize].branches[0] = Compartment {
        branch_type: 1, // Any branch type > 0 works
        refractory_timer: 0,
        voltage: pack_voltage(0.0),
        nmda_threshold: pack_voltage(20.0), // Threshold = 20
        local_trace: 0,
            _pad: 0,
    };
    
    // Send a signal above threshold to Proximal branch (0) so it triggers massive somatic voltage
    graph.neurons[id as usize].branches[0].voltage = pack_voltage(25.0);
    graph.tick(1, env.clone());
    
    assert!(graph.neurons[id as usize].current_voltage > initial_voltage + 50.0, "NMDA spike should inject massive voltage into soma");
}

#[test]
fn test_branch_isolation() {
    let mut graph = NetworkGraph::new();
    let id = graph.add_neuron(NeuronType::Pyramidal);
    let env = default_env();
    
    graph.neurons[id as usize].branches[1] = Compartment {
        branch_type: 1,
        refractory_timer: 0,
        voltage: pack_voltage(50.0),
        nmda_threshold: pack_voltage(100.0),
        local_trace: 0,
            _pad: 0,
    };
    
    graph.neurons[id as usize].branches[2] = Compartment {
        branch_type: 1,
        refractory_timer: 0,
        voltage: pack_voltage(0.0),
        nmda_threshold: pack_voltage(100.0),
        local_trace: 0,
            _pad: 0,
    };
    
    graph.tick(1, env.clone());
    
    let v2 = unpack_voltage(graph.neurons[id as usize].branches[2].voltage);
    assert!(v2.abs() < 0.1, "Branch 2 should remain isolated from Branch 1");
}

#[test]
fn test_trunk_vs_tuft_penalty() {
    let mut graph_trunk = NetworkGraph::new();
    let mut graph_tuft = NetworkGraph::new();
    let id_trunk = graph_trunk.add_neuron(NeuronType::Pyramidal);
    let id_tuft = graph_tuft.add_neuron(NeuronType::Pyramidal);
    let env = default_env();
    
    graph_trunk.neurons[id_trunk as usize].energy_atp = 100.0;
    graph_tuft.neurons[id_tuft as usize].energy_atp = 100.0;
    
    graph_trunk.neurons[id_trunk as usize].branches[1] = Compartment {
        branch_type: 2, // Trunk
        refractory_timer: 0,
        voltage: pack_voltage(10.0),
        nmda_threshold: pack_voltage(100.0),
        local_trace: 0,
            _pad: 0,
    };
    
    graph_tuft.neurons[id_tuft as usize].branches[1] = Compartment {
        branch_type: 3, // Tuft
        refractory_timer: 0,
        voltage: pack_voltage(10.0),
        nmda_threshold: pack_voltage(100.0),
        local_trace: 0,
            _pad: 0,
    };
    
    graph_trunk.tick(1, env.clone());
    graph_tuft.tick(1, env.clone());
    
    let atp_trunk = graph_trunk.neurons[id_trunk as usize].energy_atp;
    let atp_tuft = graph_tuft.neurons[id_tuft as usize].energy_atp;
    
    assert!(atp_trunk < atp_tuft, "Trunk should consume more ATP than Tuft");
}

#[test]
fn test_branch_refractory_period() {
    let mut graph = NetworkGraph::new();
    let id = graph.add_neuron(NeuronType::Pyramidal);
    let env = default_env();
    
    graph.neurons[id as usize].branches[1] = Compartment {
        branch_type: 1,
        refractory_timer: 0,
        voltage: pack_voltage(30.0),
        nmda_threshold: pack_voltage(20.0), // Force NMDA spike
        local_trace: 0,
            _pad: 0,
    };
    
    graph.tick(1, env.clone()); // NMDA spike happens here
    
    let timer = graph.neurons[id as usize].branches[1].refractory_timer;
    assert_eq!(timer, 3, "Branch should enter refractory period of 3 ticks");
    
    // Tick 2: Should be refractory, ignores new voltage
    graph.neurons[id as usize].branches[1].voltage = pack_voltage(50.0);
    graph.tick(2, env.clone());
    let v_ref = unpack_voltage(graph.neurons[id as usize].branches[1].voltage);
    assert!(v_ref < 1.0, "Voltage should be reset during refractory period");
    assert_eq!(graph.neurons[id as usize].branches[1].refractory_timer, 2);
}

#[test]
fn test_passive_leak_diffusion() {
    let mut graph = NetworkGraph::new();
    let id = graph.add_neuron(NeuronType::Pyramidal);
    let env = default_env();
    
    graph.neurons[id as usize].current_voltage = -70.0;
    
    graph.neurons[id as usize].branches[1] = Compartment {
        branch_type: 1,
        refractory_timer: 0,
        voltage: pack_voltage(10.0), 
        nmda_threshold: pack_voltage(100.0),
        local_trace: 0,
            _pad: 0,
    };
    
    graph.tick(1, env.clone());
    
    let soma_v = graph.neurons[id as usize].current_voltage;
    println!("Soma V after leak: {}", soma_v);
    
    // In evaluate_branches:
    // v = 10.0 * 0.9 = 9.0
    // passive_leak = 9.0 * 0.05 = 0.45
    // n->current_voltage += 0.45  => -69.55
    // But then there's VOLTAGE_DECAY_RATE (0.1) in process_spike if we were above resting voltage, but we are below resting voltage? No, resting is -70.
    // Wait, process_spike might pull it back towards resting voltage, or ATP drain might happen.
    
    assert!(soma_v > -70.0, "Soma should receive a small passive leak charge");
}

#[test]
fn test_dendritic_role_separation() {
    let mut graph = NetworkGraph::new();
    graph.add_neuron(NeuronType::Pyramidal); // 0 (Source for Proximal)
    graph.add_neuron(NeuronType::Pyramidal); // 1 (Source for Distal)
    graph.add_neuron(NeuronType::Pyramidal); // 2 (Target)

    let mut env = Environment {
        global_oxygen_level: 1.0,
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0,
        toxin_levels: 0.0,
        light_level: 0.0,
        stdp_learning_rate: 0.1,
        wakefulness_multiplier: 1.0,
    };

    // Connect Source 0 to Proximal (Branch 2)
    graph.synapses.push(Synapse {
        source_id: 0,
        target_id: 2,
        weight: 5.0,
        neurotransmitter_buffer: 0.0,
        last_activation_tick: 0,
        ticks_below_threshold: 0,
        ticks_until_delivery: 0,
        flags: 2, // Branch 2 (Proximal)
        _padding: [0; 2],
    });

    // Connect Source 1 to Distal (Branch 6)
    graph.synapses.push(Synapse {
        source_id: 1,
        target_id: 2,
        weight: 5.0,
        neurotransmitter_buffer: 0.0,
        last_activation_tick: 0,
        ticks_below_threshold: 0,
        ticks_until_delivery: 0,
        flags: 6, // Branch 6 (Distal)
        _padding: [0; 2],
    });

    graph.optimize_memory_layout();
    graph.force_sync_gpu();

    let target_resting = graph.neurons[2].resting_voltage;

    // Phase 1: Fire Distal (Branch 6)
    graph.neurons[1].current_voltage = 50.0;
    graph.tick(1, env);
    graph.tick(2, env); // Delivery
    graph.tick(3, env); // Summation

    // Distal should set Predictive State, but NOT depolarize significantly
    assert_eq!(graph.neurons[2].predictive_ticks, 20, "Distal branch must trigger Predictive State");
    assert!(graph.neurons[2].current_voltage < target_resting + 1.0, "Distal should not cause huge somatic depolarization");

    // Phase 2: Fire Proximal (Branch 2)
    graph.neurons[0].current_voltage = 50.0;
    graph.tick(4, env);
    graph.tick(5, env); // Delivery
    graph.tick(6, env); // Summation

    // Proximal should depolarize the soma immediately
    assert!(graph.neurons[2].current_voltage > target_resting + 10.0, "Proximal must cause immediate somatic depolarization");
}
