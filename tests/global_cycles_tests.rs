use neuromorphic_engine::graph::NetworkGraph;
use neuromorphic_engine::ffi::{Environment, NeuronType, Synapse, Compartment};
use neuromorphic_engine::constants::*;

fn default_env() -> Environment {
    Environment {
        global_oxygen_level: 1.0,
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0,
        toxin_levels: 0.0,
        light_level: 0.0, stdp_learning_rate: 0.1,
    }
}

fn pack_voltage(v: f32) -> u16 {
    let mut shifted = v + 100.0;
    if shifted < 0.0 { shifted = 0.0; }
    if shifted > 65535.0 / 100.0 { shifted = 65535.0 / 100.0; }
    (shifted * 100.0) as u16
}

#[test]
fn test_fatigue_sleep_trigger() {
    let mut graph = NetworkGraph::new();
    for _ in 0..100 {
        graph.add_neuron(NeuronType::Pyramidal);
    }
    
    for n in &mut graph.neurons {
        n.metabolic_wear = 65000; // Above 60_000 threshold
    }
    
    assert!(graph.is_sleep_needed(), "Sleep should be triggered when metabolic wear is high");
}

#[test]
fn test_synapse_pruning() {
    let mut graph = NetworkGraph::new();
    let src = graph.add_neuron(NeuronType::Pyramidal);
    let tgt = graph.add_neuron(NeuronType::Pyramidal);
    
    graph.synapses.push(Synapse {
        source_id: src,
        target_id: tgt,
        weight: 0.005, 
        neurotransmitter_buffer: 0.0,
        last_activation_tick: 0,
        ticks_below_threshold: 0,
        ticks_until_delivery: 0,
        flags: 0,
        _padding: [0; 2],
    });
    
    graph.synapses.push(Synapse {
        source_id: tgt,
        target_id: src,
        weight: 0.5, 
        neurotransmitter_buffer: 0.0,
        last_activation_tick: 0,
        ticks_below_threshold: 0,
        ticks_until_delivery: 0,
        flags: 0,
        _padding: [0; 2],
    });
    
    assert_eq!(graph.synapses.len(), 2);
    graph.initiate_sleep_cycle();
    
    assert_eq!(graph.synapses.len(), 1, "Weak synapse should be pruned during sleep");
    assert_eq!(graph.synapses[0].weight, 0.5, "Strong synapse should survive pruning");
}

#[test]
fn test_neurogenesis_rate() {
    let mut graph = NetworkGraph::new();
    for _ in 0..10 {
        let id = graph.add_neuron(NeuronType::Pyramidal);
        graph.neurons[id as usize].x = 1000;
        graph.neurons[id as usize].y = 1000;
        graph.neurons[id as usize].z = 1000;
        graph.neurons[id as usize].last_spike_tick = 400; // active in window
    }
    
    let initial_synapses = graph.synapses.len();
    
    // Neurogenesis is highly stochastic (1% per check, checks = 2*N = 20 per call)
    // Run it enough times to guarantee a hit
    for _ in 0..500 {
        graph.run_neurogenesis_service(500); // Pass 500 so `tick % NEUROGENESIS_INTERVAL == 0` is true
    }
    
    let final_synapses = graph.synapses.len();
    assert!(final_synapses > initial_synapses, "Neurogenesis should create new synapses over time between active, close neurons");
    
    let mut counts = vec![0; 10];
    for s in &graph.synapses {
        counts[s.source_id as usize] += 1;
    }
    
    for &c in &counts {
        assert!(c <= MAX_SYNAPSES_PER_NEURON, "Neuron exceeded max synapses limit");
    }
}

#[test]
fn test_dopamine_decay() {
    use neuromorphic_engine::stimulus::StimulusService;
    let mut env = default_env();
    let mut graph = NetworkGraph::new();
    graph.add_neuron(NeuronType::Pyramidal);
    let mut stim = StimulusService::new(1000);
    
    env.dopamine_multiplier = 8.0; 
    
    let mut ticks_to_decay = 0;
    for tick in 1..=200 {
        if env.dopamine_multiplier <= 1.0 { break; }
        stim.process_tick(&mut graph, &mut env, tick);
        ticks_to_decay += 1;
    }
    
    assert!(ticks_to_decay >= 69 && ticks_to_decay <= 71, "Dopamine should decay from 8.0 to 1.0 in roughly 70 ticks");
}

#[test]
fn test_pavlovian_success_rate() {
    let mut graph = NetworkGraph::new();
    let sensor = graph.add_neuron(NeuronType::Pyramidal);
    let relay = graph.add_neuron(NeuronType::Pyramidal);
    let motor = graph.add_neuron(NeuronType::Pyramidal);
    let mut env = default_env();
    
    graph.synapses.push(Synapse {
        source_id: sensor,
        target_id: relay,
        weight: 1.0, 
        neurotransmitter_buffer: 0.0,
        last_activation_tick: 0,
        ticks_below_threshold: 0,
        ticks_until_delivery: 0,
        flags: 1, // Branch 1
        _padding: [0; 2],
    });
    
    graph.synapses.push(Synapse {
        source_id: relay,
        target_id: motor,
        weight: 1.0, 
        neurotransmitter_buffer: 0.0,
        last_activation_tick: 0,
        ticks_below_threshold: 0,
        ticks_until_delivery: 0,
        flags: 1, // Branch 1
        _padding: [0; 2],
    });
    
    graph.neurons[relay as usize].branches[1] = Compartment {
        branch_type: 1,
        refractory_timer: 0,
        voltage: pack_voltage(0.0),
        nmda_threshold: pack_voltage(20.0),
        _padding: [0; 2],
    };
    
    graph.neurons[motor as usize].branches[1] = Compartment {
        branch_type: 1,
        refractory_timer: 0,
        voltage: pack_voltage(0.0),
        nmda_threshold: pack_voltage(20.0),
        _padding: [0; 2],
    };
    
    // Train the network
    for tick in 1..=500 {
        env.dopamine_multiplier = 4.0; 
        
        graph.neurons[sensor as usize].energy_atp = 100.0;
        graph.neurons[relay as usize].energy_atp = 100.0;
        graph.neurons[motor as usize].energy_atp = 100.0;
        
        if tick % 10 == 1 {
            graph.neurons[sensor as usize].current_voltage = 40.0;
            graph.neurons[sensor as usize].last_spike_tick = tick;
        }
        if tick % 10 == 3 {
            graph.neurons[relay as usize].current_voltage = 40.0;
            graph.neurons[relay as usize].last_spike_tick = tick;
        }
        if tick % 10 == 5 {
            graph.neurons[motor as usize].current_voltage = 40.0;
            graph.neurons[motor as usize].last_spike_tick = tick;
        }
        
        graph.tick(tick, env.clone());
        
        // Prevent seizure loop by wiping chemical/voltage accumulations during forced training
        graph.synapses[0].neurotransmitter_buffer = 0.0;
        graph.synapses[1].neurotransmitter_buffer = 0.0;
        graph.neurons[relay as usize].branches[1].voltage = pack_voltage(0.0);
        graph.neurons[motor as usize].branches[1].voltage = pack_voltage(0.0);

        if tick % 10 == 0 {
            graph.run_astrocyte_service(tick);
        }
    }
    
    let w1 = graph.synapses[0].weight;
    let w2 = graph.synapses[1].weight;
    
    assert!(w1 > 1.0, "Sensor->Relay synapse should be potentiated");
    assert!(w2 > 1.0, "Relay->Motor synapse should be potentiated");
    
    // Test the pathway
    env.dopamine_multiplier = 1.0;
    
    graph.neurons[sensor as usize].current_voltage = -70.0;
    graph.neurons[relay as usize].current_voltage = -70.0;
    graph.neurons[motor as usize].current_voltage = -70.0;
    
    // Reset refractory
    graph.neurons[sensor as usize].refractory_time_left = 0.0;
    graph.neurons[relay as usize].refractory_time_left = 0.0;
    graph.neurons[motor as usize].refractory_time_left = 0.0;
    
    for tick in 51..=60 { graph.tick(tick, env.clone()); }
    
    graph.neurons[sensor as usize].energy_atp = 100.0;
    graph.neurons[relay as usize].energy_atp = 100.0;
    graph.neurons[motor as usize].energy_atp = 100.0;
    
    graph.neurons[sensor as usize].current_voltage = 45.0; // Above threshold
    
    for tick in 61..=100 {
        graph.neurons[relay as usize].energy_atp = 100.0;
        graph.neurons[motor as usize].energy_atp = 100.0;
        graph.tick(tick, env.clone());
    }
    
    assert!(graph.neurons[motor as usize].last_spike_tick > 60, "Motor should fire automatically after training when sensor is activated");
}
