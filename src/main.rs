pub mod streamer;
pub mod ffi;
pub mod graph;
pub mod constants;
pub mod telemetry;
pub mod stimulus;
pub mod modes;
pub mod visualization;

use crate::constants::*;
use crate::modes::*;
use crate::visualization::*;
use crate::graph::NetworkGraph;
use tokio::sync::broadcast;
use std::env;

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

    let (tx, _rx) = broadcast::channel(100);
    
    // Spawn WebSocket server only if --ui flag is present
    if is_ui_enabled {
        println!("UI Mode enabled. Starting WebSocket server...");
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            crate::streamer::start_server(tx_clone).await;
        });
        // Give the server a moment to actually bind the port before we hammer the CPU
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    } else {
        println!("UI Mode disabled. Running in headless high-performance mode (use --ui to enable visualizer).");
    }

    let tx_option = if is_ui_enabled { Some(tx.clone()) } else { None };

    if is_bench_scaling {
        modes::run_bench_scaling().await?;
    } else if is_bench_io {
        modes::run_bench_io().await?;
    } else if is_sleep_test {
        run_sleep_test()?;
        return Ok(());
    }

    if is_benchmark {
        let mut neurons = DEFAULT_BENCHMARK_NEURONS;
        let mut synapses = DEFAULT_BENCHMARK_SYNAPSES;
        let mut ticks = DEFAULT_BENCHMARK_TICKS;

        for i in 1..args.len() {
            if args[i] == "--neurons" && i + 1 < args.len() {
                neurons = args[i+1].parse().unwrap_or(neurons);
            }
            if args[i] == "--synapses" && i + 1 < args.len() {
                synapses = args[i+1].parse().unwrap_or(synapses);
            }
            if args[i] == "--ticks" && i + 1 < args.len() {
                ticks = args[i+1].parse().unwrap_or(ticks);
            }
        }

        // Force fresh start for benchmark if requested
        if args.contains(&String::from("--fresh")) {
            if std::path::Path::new("brain_save.bin").exists() {
                println!("Deleting old brain_save.bin for a fresh start...");
                let _ = std::fs::remove_file("brain_save.bin");
            }
        }

        let data = run_benchmark_mode(neurons, synapses, ticks, tx_option).await?;
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

    let graph = if let Some(loaded_graph) = NetworkGraph::load_from_disk() {
        println!("Awakening network from brain_save.bin for simulation...");
        loaded_graph
    } else {
        setup_simulation()
    };
    let data = run_simulation(graph, DEFAULT_SIMULATION_TICKS, tx_option).await;
    render_chart(&data, "graphs/phase8_sensors.png")?;

    Ok(())
}
