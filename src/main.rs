

use neuromorphic_engine::engine::constants::*;
use neuromorphic_engine::runner::modes::*;
use neuromorphic_engine::monitor::visualization::*;
use neuromorphic_engine::network::graph::NetworkGraph;
use tokio::sync::broadcast;
use std::env;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Initializing neuromorphic-engine Hybrid Engine...");

    let args: Vec<String> = env::args().collect();
    let is_ui_enabled = args.contains(&String::from("--ui"));
    let is_benchmark = args.contains(&String::from("--benchmark"));
    let is_stress_test = args.contains(&String::from("--stress-test"));
    let is_dopamine_rush = args.contains(&String::from("--dopamine-rush"));
    let is_reflex_arc = args.contains(&String::from("--reflex-arc"));
    let is_inhibition_loop = args.contains(&String::from("--inhibition-loop"));
    let is_scaling_test = args.contains(&String::from("--scaling-test"));
    let is_frequency_test = args.contains(&String::from("--frequency-test"));
    let is_emergence_test = args.contains(&String::from("--emergence-test"));
    let is_morphology_audit = args.contains(&String::from("--morphology-audit"));
    let is_spatial_test = args.contains(&String::from("--spatial-test"));
    let is_passive_leak_test = args.contains(&String::from("--passive-leak-test"));
    let is_sleep_test = args.contains(&String::from("--sleep-test"));
    let is_bench_scaling = args.contains(&String::from("--bench-scaling"));
    let is_bench_io = args.contains(&String::from("--bench-io"));
    let is_ui_test = args.contains(&String::from("--ui-test"));

    let mut parsed_neurons = DEFAULT_BENCHMARK_NEURONS;
    let mut parsed_synapses = DEFAULT_BENCHMARK_SYNAPSES;
    let mut parsed_ticks = DEFAULT_SIMULATION_TICKS;

    for i in 1..args.len() {
        if args[i] == "--neurons" && i + 1 < args.len() {
            parsed_neurons = args[i+1].parse().unwrap_or(parsed_neurons);
        }
        if args[i] == "--synapses" && i + 1 < args.len() {
            parsed_synapses = args[i+1].parse().unwrap_or(parsed_synapses);
        }
        if args[i] == "--ticks" && i + 1 < args.len() {
            parsed_ticks = args[i+1].parse().unwrap_or(parsed_ticks);
        }
    }

    let (tx, _rx) = broadcast::channel(100);

    // Shared Environment for UI updates
    let default_env = neuromorphic_engine::engine::ffi::Environment {
        global_oxygen_level: 100.0,
        global_glucose_level: 100.0,
        dopamine_multiplier: 1.0,
        toxin_levels: 0.0,
        light_level: 0.0,
        stdp_learning_rate: 0.1,
    };
    let shared_env = Arc::new(Mutex::new(default_env));

    
    let tx_option = if is_ui_enabled || is_ui_test {
        println!("Spawning WebSocket server on port 3030...");
        let tx_clone = tx.clone();
        let env_clone = shared_env.clone();
        tokio::spawn(async move {
            neuromorphic_engine::monitor::streamer::start_server(tx_clone, env_clone).await;
        });
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        Some(tx)
    } else {
        println!("UI Mode disabled. Running in headless high-performance mode (use --ui to enable visualizer).");
        None
    };

    if is_bench_scaling {
        run_bench_scaling().await?;
    } else if is_bench_io {
        run_bench_io().await?;
    } else if is_ui_test {
        run_ui_test(tx_option.clone(), shared_env.clone()).await?;
        println!("Simulation finished. Keeping WebSocket server alive for UI. Press Ctrl+C to exit.");
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }
    } else if is_sleep_test {
        run_sleep_test()?;
        return Ok(());
    }

    if is_benchmark {
        // Force fresh start for benchmark if requested
        if args.contains(&String::from("--fresh")) {
            if std::path::Path::new("brain_save.bin").exists() {
                println!("Deleting old brain_save.bin for a fresh start...");
                let _ = std::fs::remove_file("brain_save.bin");
            }
        }

        if tx_option.is_some() {
            println!("\n=======================================================");
            println!("🚀 UI MODE ACTIVE: Open http://127.0.0.1:3030/index.html");
            println!("⏳ Waiting 5 seconds for WebSocket connection before starting ticks...");
            println!("=======================================================\n");
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }

        let data = run_benchmark_mode(parsed_neurons, parsed_synapses, parsed_ticks, tx_option, shared_env.clone(), true).await?;
        render_benchmark_chart(&data, "graphs/benchmark_load.png")?;
        return Ok(());
    }

    if is_scaling_test {
        run_scaling_benchmarks().await?;
        return Ok(());
    }

    if is_emergence_test {
        run_emergence_test_mode()?;
        return Ok(());
    }

    if is_morphology_audit {
        run_morphology_audit()?;
        return Ok(());
    }

    if is_spatial_test {
        run_spatial_summation_test()?;
        return Ok(());
    }

    if is_passive_leak_test {
        run_passive_leak_test()?;
        return Ok(());
    }

    if is_frequency_test {
        run_frequency_test_mode()?;
        return Ok(());
    }

    if is_stress_test {
        run_stress_test_mode()?;
        return Ok(());
    }

    if is_dopamine_rush {
        run_dopamine_rush_mode()?;
        return Ok(());
    }

    if is_reflex_arc {
        run_reflex_arc_mode()?;
        return Ok(());
    }

    if is_inhibition_loop {
        run_inhibition_loop_mode()?;
        return Ok(());
    }

    let is_fresh = args.contains(&String::from("--fresh"));
    if is_fresh && std::path::Path::new("brain_save.bin").exists() {
        println!("Deleting old brain_save.bin for a fresh start...");
        let _ = std::fs::remove_file("brain_save.bin");
    }

    let graph = if let Some(loaded_graph) = NetworkGraph::load_from_disk() {
        println!("Awakening network from brain_save.bin for simulation...");
        loaded_graph
    } else if parsed_neurons > 2 {
        setup_benchmark(parsed_neurons, parsed_synapses)
    } else {
        setup_simulation()
    };
    
    if tx_option.is_some() {
        println!("\n=======================================================");
        println!("🚀 UI MODE ACTIVE: Open http://127.0.0.1:3030/index.html");
        println!("⏳ Waiting 5 seconds for WebSocket connection before starting ticks...");
        println!("=======================================================\n");
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }

    let data = run_simulation(graph, parsed_ticks, tx_option.clone(), true).await;
    render_chart(&data, "graphs/phase8_sensors.png")?;

    if is_ui_enabled || is_ui_test {
        println!("Simulation finished. Keeping WebSocket server alive for UI. Press Ctrl+C to exit.");
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }
    }

    Ok(())
}
