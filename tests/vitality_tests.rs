use neuromorphic_engine::engine::ffi::{BaseNeuron, Environment, NeuronType, Compartment};
use neuromorphic_engine::engine::environment::EnvironmentRegulator;

// For FFI tests
extern "C" {
    fn tick_neuron_cpu(neuron: *mut BaseNeuron, env: Environment, current_tick: u32);
    fn init_neuron(neuron: *mut BaseNeuron, id: u32, type_: NeuronType, seed: u32);
}

fn create_test_neuron(id: u32, type_: NeuronType) -> BaseNeuron {
    let mut n = BaseNeuron {
        id: 0, type_: 0, death_reason: 0, is_alive: 0, internal_phase: 0,
        spike_trace: 0.0, current_voltage: 0.0, activation_threshold: 0.0, resting_voltage: 0.0,
        energy_atp: 0.0, max_atp: 0.0, leak_coeff: 0.0, calcium_level: 0.0, vesicles_pool: 0.0,
        refractory_time_left: 0.0, last_spike_tick: 0, spikes_in_last_window: 0, predictive_ticks: 0, metabolic_wear: 0,
        x: 0, y: 0, z: 0, dead_ticks: 0,
        column_id: 0,
        minicolumn_id: 0,
        layer_id: 0,
        status_flags: 0,
        _pad_block1: [0; 32],
        ref_coord_x: 0.0,
        ref_coord_y: 0.0,
        _pad_block2: [0; 16],
        branches: [Compartment { 
            branch_type: 0, refractory_timer: 0, voltage: 0, nmda_threshold: 0, local_trace: 0, _pad: 0 
        }; 16],
    };
    unsafe { init_neuron(&mut n, id, type_, 42); }
    n
}

#[test]
fn test_circadian_startup_grace() {
    let mut regulator = EnvironmentRegulator::new();
    let base_env = Environment {
        global_oxygen_level: 1.0,
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0,
        toxin_levels: 0.0,
        light_level: 0.0,
        stdp_learning_rate: 0.1,
        wakefulness_multiplier: 1.0,
    };

    // First 100 ticks (inside 5000 grace period) - massive lag, should not drop oxygen
    let env_tick_100 = regulator.update_and_get(500.0, 100, &base_env);
    assert_eq!(env_tick_100.global_oxygen_level, 1.0, "Oxygen must remain 1.0 during startup grace period despite lag");
    assert_eq!(env_tick_100.global_glucose_level, 1.0, "Glucose must remain 1.0 during startup grace period despite lag");
    assert!(env_tick_100.wakefulness_multiplier > 1.4, "Wakefulness should be high during early ticks");

    // Tick 6000 (outside grace period) - massive lag, oxygen must drop
    let env_tick_6000 = regulator.update_and_get(500.0, 6000, &base_env);
    assert_eq!(env_tick_6000.global_oxygen_level, 1.0, "Oxygen should remain 1.0 due to Phase 18 Metabolic Equilibrium Fix");
    assert!(env_tick_6000.wakefulness_multiplier < env_tick_100.wakefulness_multiplier, "Wakefulness must decay over time");
}

#[test]
fn test_thermal_noise_prevents_freezing() {
    let mut neuron = create_test_neuron(1, NeuronType::Pyramidal); // Pyramidal
    neuron.current_voltage = neuron.resting_voltage;
    
    let env = Environment {
        global_oxygen_level: 1.0,
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0,
        toxin_levels: 0.0,
        light_level: 0.0,
        stdp_learning_rate: 0.1,
        wakefulness_multiplier: 1.0,
    };

    // Tick it 50 times in a vacuum
    for tick in 1..=50 {
        unsafe { tick_neuron_cpu(&mut neuron, env.clone(), tick); }
    }

    // Since we apply thermal noise, voltage should NOT be exactly the resting voltage.
    assert!((neuron.current_voltage - neuron.resting_voltage).abs() > 0.0001, "Thermal noise should perturb voltage away from perfect rest");
}

#[test]
fn test_phoenix_resurrection() {
    let mut neuron = create_test_neuron(2, NeuronType::Pyramidal);
    // Force it to die
    neuron.is_alive = 0;
    neuron.energy_atp = 0.0;
    neuron.dead_ticks = 0;
    
    let env = Environment {
        global_oxygen_level: 1.0,
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0,
        toxin_levels: 0.0,
        light_level: 0.0,
        stdp_learning_rate: 0.1,
        wakefulness_multiplier: 1.0,
    };

    // Tick 999 times - should remain dead
    for tick in 1..=999 {
        unsafe { tick_neuron_cpu(&mut neuron, env.clone(), tick); }
        assert_eq!(neuron.is_alive, 0, "Neuron should remain dead at tick {}", tick);
    }
    
    // Tick 1000 - Resurrection!
    unsafe { tick_neuron_cpu(&mut neuron, env.clone(), 1000); }
    
    assert_eq!(neuron.is_alive, 1, "Phoenix loop must resurrect neuron at 1000 ticks");
    assert_eq!(neuron.energy_atp, neuron.max_atp, "Resurrected neuron must have full ATP");
    assert_eq!(neuron.dead_ticks, 0, "Dead ticks counter must reset");
}
