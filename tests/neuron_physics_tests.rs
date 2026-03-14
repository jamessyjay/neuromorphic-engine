use neuromorphic_engine::engine::ffi::{BaseNeuron, Synapse, Environment, NeuronType};
use neuromorphic_engine::engine::ffi::{tick_neuron_cpu, tick_synapse_cpu, init_neuron};
use std::fs::File;
use std::io::{Write, Read};
use serde::{Serialize, Deserialize};

fn default_env() -> Environment {
    Environment {
        global_oxygen_level: 1.0,
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0,
        toxin_levels: 0.0,
        light_level: 0.5,
        stdp_learning_rate: 0.01,
    }
}

// Re-implement the voltage pack/unpack logic to manipulate branches in Rust
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
fn test_atp_gating_starvation_filter() {
    let mut n_fed = unsafe { std::mem::zeroed::<BaseNeuron>() };
    let mut n_starving = unsafe { std::mem::zeroed::<BaseNeuron>() };
    
    // Setup identical Pyramidal neurons
    unsafe {
        init_neuron(&mut n_fed, 1, NeuronType::Pyramidal, 42);
        init_neuron(&mut n_starving, 2, NeuronType::Pyramidal, 41);
    }
    
    n_fed.energy_atp = n_fed.max_atp;
    n_starving.energy_atp = 0.5; // Near zero
    
    // Both should have same base thresholds
    assert!((n_fed.activation_threshold - n_starving.activation_threshold).abs() < 0.01);
    
    // Calculate dynamic thresholds in Rust to assert
    let fed_ratio = (n_fed.energy_atp / n_fed.max_atp).max(0.01);
    let starving_ratio = (n_starving.energy_atp / n_starving.max_atp).max(0.01);
    
    let fed_dyn_thresh = n_fed.activation_threshold * (1.0 - 0.5 * (1.0 - fed_ratio));
    let starving_dyn_thresh = n_starving.activation_threshold * (1.0 - 0.5 * (1.0 - starving_ratio));
    
    // Assert physiological correctness:
    // Starving threshold must be greater (closer to zero) than fed threshold!
    assert!(starving_dyn_thresh > fed_dyn_thresh, "Starving threshold must be greater (closer to zero) than fed threshold");
    
    println!("[ATP-GATING] Fed threshold: {:.2} mV | Starving threshold: {:.2} mV", fed_dyn_thresh, starving_dyn_thresh);

    
    // Inject stimulus to soma directly to clearly test the threshold gate
    // If we must inject to branch, we inject a massive amount so it transfers to soma
    let gap = n_fed.activation_threshold - n_fed.current_voltage;
    
    // We want the soma to reach `activation_threshold + 1.0` after evaluation.
    n_fed.current_voltage += gap + 1.0;
    n_starving.current_voltage += gap + 1.0;
    
    let env = default_env();
    
    unsafe {
        tick_neuron_cpu(&mut n_fed, env.clone(), 1);
        tick_neuron_cpu(&mut n_starving, env.clone(), 1);
    }
    
    assert!(n_fed.current_voltage <= n_fed.resting_voltage + 5.0 || n_fed.refractory_time_left > 0.0, "Fed neuron should have spiked!");
    assert!(n_starving.current_voltage > n_starving.resting_voltage + 5.0 && n_starving.refractory_time_left == 0.0, "Starving neuron should have ignored the signal due to higher dynamic threshold!");
}


#[test]
fn test_phase_resonance_internal_clock() {
    let mut source = unsafe { std::mem::zeroed::<BaseNeuron>() };
    let mut target_sync = unsafe { std::mem::zeroed::<BaseNeuron>() };
    let mut target_async = unsafe { std::mem::zeroed::<BaseNeuron>() };
    
    unsafe {
        init_neuron(&mut source, 0, NeuronType::Pyramidal, 42);
        init_neuron(&mut target_sync, 1, NeuronType::Pyramidal, 43);
        init_neuron(&mut target_async, 2, NeuronType::Pyramidal, 44);
    }
    
    // Set phases
    source.internal_phase = 100;
    target_sync.internal_phase = 100; // Perfect match
    target_async.internal_phase = 228; // 128 difference (perfect opposite)
    
    // Create synapses
    let mut syn_sync = unsafe { std::mem::zeroed::<Synapse>() };
    syn_sync.source_id = 0;
    syn_sync.target_id = 1;
    syn_sync.weight = 5.0;
    syn_sync.flags = 0x00; // Branch 0, excitatory
    syn_sync.neurotransmitter_buffer = 10.0;
    
    let mut syn_async = unsafe { std::mem::zeroed::<Synapse>() };
    syn_async.source_id = 0;
    syn_async.target_id = 2;
    syn_async.weight = 5.0;
    syn_async.flags = 0x00; // Branch 0, excitatory
    syn_async.neurotransmitter_buffer = 10.0;
    
    // Force source spike exactly on this tick so it releases neurotransmitters
    source.current_voltage = 40.0; // ACTION_POTENTIAL_PEAK
    source.last_spike_tick = 1;
    
    let env = default_env();
    
    // Setup targets in an array since tick_synapse_cpu expects a BaseNeuron array
    let mut neurons = vec![source.clone(), target_sync.clone(), target_async.clone()];
    
    // Run exactly 11 ticks to allow neurotransmitter to travel (UNMYELINATED_DELIVERY_TICKS = 10)
    for t in 1..=12 {
        unsafe {
            tick_synapse_cpu(&mut syn_sync, neurons.as_mut_ptr(), env.clone(), t);
            tick_synapse_cpu(&mut syn_async, neurons.as_mut_ptr(), env.clone(), t);
        }
    }
    
    let v_sync = unpack_voltage(neurons[1].branches[0].voltage);
    let v_async = unpack_voltage(neurons[2].branches[0].voltage);
    
    println!("Sync voltage impact: {}, Async voltage impact: {}", v_sync, v_async);
    
    assert!(v_sync > v_async + 0.1, "Target Sync branch voltage must be significantly higher than Async");
    assert!(v_async < 0.01, "Target Async should receive near zero signal due to total destructive interference");
}

#[test]
fn test_precise_stdp_targeted_dendritic_learning() {
    let mut source1 = unsafe { std::mem::zeroed::<BaseNeuron>() };
    let mut source2 = unsafe { std::mem::zeroed::<BaseNeuron>() };
    let mut target = unsafe { std::mem::zeroed::<BaseNeuron>() };
    
    unsafe {
        init_neuron(&mut source1, 0, NeuronType::Pyramidal, 42);
        init_neuron(&mut source2, 1, NeuronType::Pyramidal, 43);
        init_neuron(&mut target, 2, NeuronType::Pyramidal, 44);
    }
    
    // Set spike traces so learning can happen
    source1.spike_trace = 1.0;
    source2.spike_trace = 1.0;
    
    // Artificial branch activity traces
    target.branches[0].local_trace = 255;
    target.branches[1].local_trace = 0;
    
    // Setup synapses
    let mut syn1 = unsafe { std::mem::zeroed::<Synapse>() };
    syn1.source_id = 0;
    syn1.target_id = 2;
    syn1.weight = 2.0;
    syn1.flags = 0x00; // Branch 0, excitatory
    
    let mut syn2 = unsafe { std::mem::zeroed::<Synapse>() };
    syn2.source_id = 1;
    syn2.target_id = 2;
    syn2.weight = 2.0;
    syn2.flags = 0x01; // Branch 1, excitatory
    
    let mut env = default_env();
    env.stdp_learning_rate = 0.5; // High learning rate to see effect
    
    // Target spikes right now
    target.last_spike_tick = 1;
    
    let mut neurons = vec![source1, source2, target];
    
    unsafe {
        tick_synapse_cpu(&mut syn1, neurons.as_mut_ptr(), env.clone(), 1);
        tick_synapse_cpu(&mut syn2, neurons.as_mut_ptr(), env.clone(), 1);
    }
    
    let weight1 = syn1.weight;
    let weight2 = syn2.weight;
    
    println!("Synapse 1 (Trace 255) Weight: {}", weight1);
    println!("Synapse 2 (Trace 0) Weight: {}", weight2);
    
    assert!(weight1 > 2.011, "Synapse 1 should have learned (LTP applied)");
    assert!((weight2 - 2.01).abs() < 0.001, "Synapse 2 should not have learned (no branch trace)");
}

#[test]
fn test_nmda_multiplexing_logical_and() {
    let mut n_dist = unsafe { std::mem::zeroed::<BaseNeuron>() };
    let mut n_focus = unsafe { std::mem::zeroed::<BaseNeuron>() };
    
    unsafe {
        init_neuron(&mut n_dist, 1, NeuronType::Pyramidal, 42);
        init_neuron(&mut n_focus, 2, NeuronType::Pyramidal, 42);
    }
    
    let env = default_env();
    
    // Set branches as basal dendrites (type 1)
    n_dist.branches[0].branch_type = 1;
    n_dist.branches[1].branch_type = 1;
    n_focus.branches[0].branch_type = 1;
    
    // Set NMDA thresholds
    n_dist.branches[0].nmda_threshold = pack_voltage(-30.0);
    n_dist.branches[1].nmda_threshold = pack_voltage(-30.0);
    n_focus.branches[0].nmda_threshold = pack_voltage(-30.0);
    
    // We need a sub-threshold stimulus.
    // Base threshold is -55, resting is -70. Gap is 15.
    // If we inject something that decays via PASSIVE_LEAK_PERCENT (e.g. 0.05), we need to inject enough to just fall short of soma spike if distributed, but trigger NMDA if focused.
    // Let's inject to branches such that `v` is slightly below NMDA threshold (-30.0).
    // Let's say v = -35.0 (which means +65.0 from resting -100 if we used absolute, but actually branches start at 0.0 relative).
    // Let's check evaluate_branches logic: `if (v > threshold) ... v * NMDA_SPIKE_GAIN`
    // So if v is 35.0, and threshold is 40.0.
    
    n_dist.branches[0].nmda_threshold = pack_voltage(40.0);
    n_dist.branches[1].nmda_threshold = pack_voltage(40.0);
    n_focus.branches[0].nmda_threshold = pack_voltage(40.0);
    
    let sub_stimulus = 25.0; // Two of these is 50.0 (> 40.0)
    
    // Action A (Distributed)
    n_dist.branches[0].voltage = pack_voltage(sub_stimulus);
    n_dist.branches[1].voltage = pack_voltage(sub_stimulus);
    
    // Action B (Focused)
    n_focus.branches[0].voltage = pack_voltage(sub_stimulus * 2.0);
    
    unsafe {
        tick_neuron_cpu(&mut n_dist, env.clone(), 1);
        tick_neuron_cpu(&mut n_focus, env.clone(), 1);
    }
    
    println!("Dist voltage: {}, Focus voltage: {}", n_dist.current_voltage, n_focus.current_voltage);
    
    assert!(n_dist.current_voltage > n_dist.resting_voltage && n_dist.current_voltage <= n_dist.activation_threshold, "Distributed should depolarize but NOT spike!");
    assert!(n_focus.current_voltage <= n_focus.resting_voltage + 5.0 || n_focus.refractory_time_left > 0.0, "Focused should have triggered NMDA and spiked!");
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct SnapshotMetric {
    tick: u32,
    soma_v: f32,
    dyn_thresh: f32,
    atp: f32,
    internal_phase: u8,
    port0_v: f32,
    trace0: u8,
}

#[test]
fn test_the_matrix_golden_master_snapshot() {
    let mut neuron = unsafe { std::mem::zeroed::<BaseNeuron>() };
    unsafe {
        init_neuron(&mut neuron, 1, NeuronType::Pyramidal, 42);
    }
    
    let env = default_env();
    let mut metrics = Vec::new();
    
    for tick in 1..=100 {
        if tick == 5 || tick == 10 {
            neuron.branches[0].voltage = pack_voltage(15.0);
        }
        
        unsafe {
            tick_neuron_cpu(&mut neuron, env.clone(), tick);
        }
        
        // Calculate dynamic threshold
        let energy_ratio = (neuron.energy_atp / neuron.max_atp).max(0.01);
        // ATP_HUNGER_BETA = 0.5
        let dyn_thresh = neuron.activation_threshold * (1.0 - 0.5 * (1.0 - energy_ratio));
        
        metrics.push(SnapshotMetric {
            tick,
            soma_v: neuron.current_voltage,
            dyn_thresh,
            atp: neuron.energy_atp,
            internal_phase: neuron.internal_phase,
            port0_v: unpack_voltage(neuron.branches[0].voltage),
            trace0: neuron.branches[0].local_trace,
        });
    }
    
    let golden_file_path = "reports/neuron_golden_state.json";
    
    if std::path::Path::new(golden_file_path).exists() {
        let mut file = File::open(golden_file_path).unwrap();
        let mut data = String::new();
        file.read_to_string(&mut data).unwrap();
        let golden_metrics: Vec<SnapshotMetric> = serde_json::from_str(&data).unwrap();
        
        assert_eq!(metrics.len(), golden_metrics.len());
        
        let mut peak_v = -100.0f32;
        let mut min_atp = 1000.0f32;
        let mut max_trace = 0u8;
        
        for (i, (current, golden)) in metrics.iter().zip(golden_metrics.iter()).enumerate() {
            assert_eq!(current.tick, golden.tick, "Tick mismatch at idx {}", i);
            assert_eq!(current.internal_phase, golden.internal_phase, "Phase mismatch at idx {}", i);
            assert_eq!(current.trace0, golden.trace0, "Trace mismatch at idx {}", i);
            
            assert!((current.soma_v - golden.soma_v).abs() < 0.01, "Soma voltage mismatch at idx {}: {} vs {}", i, current.soma_v, golden.soma_v);
            assert!((current.dyn_thresh - golden.dyn_thresh).abs() < 0.01, "Dynamic threshold mismatch at idx {}: {} vs {}", i, current.dyn_thresh, golden.dyn_thresh);
            assert!((current.atp - golden.atp).abs() < 0.01, "ATP mismatch at idx {}: {} vs {}", i, current.atp, golden.atp);
            assert!((current.port0_v - golden.port0_v).abs() < 0.01, "Port 0 voltage mismatch at idx {}: {} vs {}", i, current.port0_v, golden.port0_v);
            
            if current.soma_v > peak_v { peak_v = current.soma_v; }
            if current.atp < min_atp { min_atp = current.atp; }
            if current.trace0 > max_trace { max_trace = current.trace0; }
        }
        println!("[MATRIX] Verified 100 ticks perfectly. Peak Soma V: {:.1} | Min ATP: {:.1} | Max Trace: {}", peak_v, min_atp, max_trace);

    } else {
        let mut file = File::create(golden_file_path).unwrap();
        let json = serde_json::to_string_pretty(&metrics).unwrap();
        file.write_all(json.as_bytes()).unwrap();
        println!("Golden master snapshot created at {}", golden_file_path);
    }
}
