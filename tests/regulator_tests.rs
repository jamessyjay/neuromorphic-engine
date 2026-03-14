use neuromorphic_engine::engine::environment::EnvironmentRegulator;
use neuromorphic_engine::engine::ffi::Environment;

fn mock_base_env() -> Environment {
    Environment {
        global_oxygen_level: 1.0,
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0,
        toxin_levels: 0.0,
        light_level: 1.0,
        stdp_learning_rate: 1.0,
        wakefulness_multiplier: 1.0
    }
}

#[test]
fn test_regulator_perfect_conditions() {
    let mut regulator = EnvironmentRegulator::new();
    let base = mock_base_env();
    
    let mut env = base.clone();
    for _ in 0..10 {
        env = regulator.update_and_get(1.0, 1000, &base); // 1.0ms is faster than TARGET_TICK_MS (2.0)
    }
    
    assert_eq!(env.global_oxygen_level, 1.0, "Oxygen should remain at 100% under perfect conditions");
    assert_eq!(env.global_glucose_level, 1.0, "Glucose should remain at 100% under perfect conditions");
}

#[test]
fn test_regulator_sudden_lag() {
    let mut regulator = EnvironmentRegulator::new();
    let base = mock_base_env();
    
    // One massive spike
    let env = regulator.update_and_get(10.0, 1000, &base); // 10.0ms hits CRITICAL_TICK_MS
    
    assert_eq!(env.global_oxygen_level, 0.0, "Oxygen must drop to 0.0 instantly on critical lag");
    assert!(env.global_glucose_level > 0.9, "Glucose should barely drop from a single spike due to EMA smoothing. Was {}", env.global_glucose_level);
}

#[test]
fn test_regulator_chronic_lag() {
    let mut regulator = EnvironmentRegulator::new();
    let base = mock_base_env();
    
    let mut env = base.clone();
    // 500 ticks of chronic lag
    for _ in 0..500 {
        env = regulator.update_and_get(10.0, 1000, &base);
    }
    
    assert_eq!(env.global_oxygen_level, 0.0, "Oxygen remains at 0.0 under continuous lag");
    assert!(env.global_glucose_level < 0.1, "Glucose should be nearly depleted after 500 ticks of chronic lag. Was {}", env.global_glucose_level);
}

#[tokio::test]
async fn test_homeostasis_loop_integration() {
    use neuromorphic_engine::engine::environment::EnvironmentRegulator;
    use neuromorphic_engine::engine::ffi::{Environment, NeuronType};
    use neuromorphic_engine::network::graph::NetworkGraph;
    use std::time::Instant;

    let mut graph = NetworkGraph::new();
    let src = graph.add_neuron(NeuronType::Pyramidal);
    let tgt = graph.add_neuron(NeuronType::Pyramidal);
    graph.connect(src, tgt, 5.0, 0, false, false);
    
    let mut regulator = EnvironmentRegulator::new();
    let mut current_env = Environment {
        global_oxygen_level: 1.0,
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0,
        toxin_levels: 0.0,
        light_level: 1.0,
        stdp_learning_rate: 1.0,
        wakefulness_multiplier: 1.0
    };
    
    let use_dynamic_homeostasis = true;

    for tick in 501..=505 {
        let tick_start = Instant::now();
        
        // Mock heavy hardware lag
        std::thread::sleep(std::time::Duration::from_millis(11));
        
        graph.tick(tick, current_env.clone());
        
        let elapsed_ms = tick_start.elapsed().as_secs_f32() * 1000.0;
        if use_dynamic_homeostasis {
            current_env = regulator.update_and_get(elapsed_ms, tick, &current_env);
        }
    }
    
    assert!(current_env.global_oxygen_level < 0.1, "Oxygen should have dropped significantly due to sleep lag!");
    println!("Loop integration test passed. Final Oxygen: {}", current_env.global_oxygen_level);
}