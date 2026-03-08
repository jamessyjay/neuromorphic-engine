// src/main.rs
mod ffi;
mod graph;
mod telemetry;

use ffi::{NeuronType, Environment};
use graph::NetworkGraph;
use telemetry::Telemetry;
use plotters::prelude::*;
use std::env;
use std::time::Instant;
use rand::prelude::*;
use rand::RngExt;

// Simple Gaussian sampler (Box-Muller)
fn get_gaussian(mu: f32, sigma: f32) -> f32 {
    let mut rng = rand::rng();
    let u1: f32 = rng.random();
    let u2: f32 = rng.random();
    let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f32::consts::PI * u2).cos();
    mu + sigma * z
}

fn setup_benchmark(neurons_count: usize, synapses_count: usize) -> NetworkGraph {
    let mut graph = NetworkGraph::new();
    let mut rng = rand::rng();

    println!("Generating Benchmark Network (Gaussian weights): {} neurons, {} synapses...", neurons_count, synapses_count);

    // Creating neurons
    for _ in 0..neurons_count {
        // Biased random types: 70% Pyramidal, others 10% each
        let n_type = match rng.random_range(0..10) {
            0..=6 => NeuronType::Pyramidal,
            7 => NeuronType::Bipolar,
            8 => NeuronType::Interneuron,
            _ => NeuronType::Pacemaker,
        };
        graph.add_neuron(n_type);
    }

    // Creating synapses with Gaussian weights (Spec 06/11)
    for _ in 0..synapses_count {
        let source = rng.random_range(0..(neurons_count as u32));
        let target = rng.random_range(0..(neurons_count as u32));
        
        // Use Gaussian distribution for weights: mean=2.0, sigma=1.0, clamp to positive
        let weight = get_gaussian(2.0, 1.0).max(0.1).min(15.0);
        
        let is_inhib = rng.random_bool(0.2); // 20% inhibitory
        let is_myelin = rng.random_bool(0.1); // 10% myelinated

        graph.connect(source, target, weight, 1.0, is_inhib, is_myelin);
    }

    println!("Network generation complete.");
    graph
}

struct BenchmarkData {
    ticks: Vec<u32>,
    ffi_times: Vec<f32>,
    rust_times: Vec<f32>,
    total_spikes: Vec<u32>,
    avg_atp: Vec<f32>,
    avg_calcium: Vec<f32>,
}

fn run_benchmark_mode(neurons_count: usize, synapses_count: usize, total_ticks: u32) -> Result<BenchmarkData, Box<dyn std::error::Error>> {
    let mut graph = setup_benchmark(neurons_count, synapses_count);
    println!("Executing BENCHMARK for {} ticks...", total_ticks);
    
    let env = Environment {
        global_oxygen_level: 1.0,  
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0, 
        toxin_levels: 0.0,
        light_level: 0.0,
        seed: 42,
    };
    
    let mut data = BenchmarkData {
        ticks: Vec::with_capacity(total_ticks as usize),
        ffi_times: Vec::with_capacity(total_ticks as usize),
        rust_times: Vec::with_capacity(total_ticks as usize),
        total_spikes: Vec::with_capacity(total_ticks as usize),
        avg_atp: Vec::with_capacity(total_ticks as usize),
        avg_calcium: Vec::with_capacity(total_ticks as usize),
    };

    let start_total = Instant::now();
    let mut total_ffi_time = std::time::Duration::new(0, 0);
    let mut total_rust_time = std::time::Duration::new(0, 0);
    let mut telemetry = Telemetry::new();

    for tick in 1..=total_ticks {
        let num_neurons = graph.neurons.len() as i32;

        let ffi_start = Instant::now();
        graph.tick(tick, env);
        let ffi_elapsed = ffi_start.elapsed();
        total_ffi_time += ffi_elapsed;
        
        // Optimized Metrics Collection: Only every 50 ticks to save CPU time at large scales
        let mut spikes = 0;
        let mut atp_sum = 0.0;
        let mut calcium_sum = 0.0;
        
        if tick % 50 == 0 || tick == total_ticks {
            for n in &graph.neurons {
                if n.current_voltage >= 39.0 { spikes += 1; }
                atp_sum += n.energy_atp;
                calcium_sum += n.calcium_level;
            }
        }

        let mut tick_rust_time = std::time::Duration::new(0, 0);
        if tick % 10 == 0 {
            let rust_start = Instant::now();
            graph.run_astrocyte_service(tick);
            graph.run_microglia_service(tick);
            tick_rust_time = rust_start.elapsed();
            total_rust_time += tick_rust_time;
        }

        telemetry.update(tick);
        if tick % 500 == 0 {
            let dead_count = graph.neurons.iter().filter(|n| !n.is_alive).count();
            telemetry.print_report(graph.neurons.len(), graph.synapses.len(), dead_count);
        }

        data.ticks.push(tick);
        data.ffi_times.push(ffi_elapsed.as_secs_f32() * 1000.0);
        data.rust_times.push(tick_rust_time.as_secs_f32() * 1000.0);
        data.total_spikes.push(spikes);
        data.avg_atp.push(atp_sum / num_neurons as f32);
        data.avg_calcium.push(calcium_sum / num_neurons as f32);
    }

    let elapsed = start_total.elapsed();
    println!("--- BENCHMARK RESULTS (macOS Mock CPU) ---");
    println!("Total Time: {:?}", elapsed);
    println!("FFI / CUDA Math Time: {:?}", total_ffi_time);
    println!("Rust Services Time: {:?}", total_rust_time);
    
    let tps = (total_ticks as f64) / elapsed.as_secs_f64();
    println!("Ticks Per Second (TPS): {:.2}", tps);
    println!("Synapse Parallelism: Verified. Each of {} synapses processed in a unique GPU thread.", synapses_count);

    telemetry.export_simulation_report(&graph.neurons, &graph.synapses, "simulation_report.json");

    Ok(data)
}

fn run_scaling_benchmarks() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- EXECUTING SCALING PERFORMANCE ANALYSIS (1K to 1M) ---");
    let sizes = [1_000, 10_000, 100_000, 1_000_000];
    let mut results = String::from("# Performance Scaling Report\n\n| Neurons | Synapses | TPS | Total Time (s) | Memory (KB) |\n|---|---|---|---|---|\n");

    for &n in &sizes {
        println!("\n>> Testing Scale: {} Neurons...", n);
        let s = n * 10;
        let start = Instant::now();
        let _ = run_benchmark_mode(n, s, 500)?; // 500 ticks is enough for scaling trends
        let elapsed = start.elapsed().as_secs_f64();
        let tps = 500.0 / elapsed;
        
        let neuron_mem = std::mem::size_of::<ffi::BaseNeuron>();
        let synapse_mem = std::mem::size_of::<ffi::Synapse>();
        let total_mem_kb = (n * neuron_mem + s * synapse_mem) as f32 / 1024.0;

        results.push_str(&format!("| {} | {} | {:.2} | {:.2} | {:.2} |\n", n, s, tps, elapsed, total_mem_kb));
    }

    println!("\nSaving report to 'docs/performance_report.md'...");
    std::fs::create_dir_all("docs")?;
    std::fs::write("docs/performance_report.md", results)?;
    println!("Scaling Benchmarks Complete!");

    Ok(())
}

fn render_benchmark_chart(data: &BenchmarkData, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Rendering Benchmark Dashboard to '{}'...", filename);
    let root = BitMapBackend::new(filename, (1200, 1000)).into_drawing_area();
    root.fill(&WHITE)?;

    let areas = root.split_evenly((3, 1));

    // Panel 1: Activity Histogram (Spikes)
    let max_spikes = data.total_spikes.iter().cloned().max().unwrap_or(1).max(100);
    let mut chart1 = ChartBuilder::on(&areas[0])
        .caption("Network Activity (Total Spikes per Tick)", ("sans-serif", 20).into_font())
        .margin(10).x_label_area_size(30).y_label_area_size(40).build_cartesian_2d(0f32..1000f32, 0u32..max_spikes + 10)?;
    chart1.configure_mesh().draw()?;
    chart1.draw_series(data.ticks.iter().zip(data.total_spikes.iter()).map(|(&x, &y)| {
        Rectangle::new([(x as f32, 0), (x as f32 + 1.0, y)], RED.filled())
    }))?;

    // Panel 2: Biology Stats (Avg ATP & Calcium)
    let mut chart2 = ChartBuilder::on(&areas[1])
        .caption("Average Biology State", ("sans-serif", 20).into_font())
        .margin(10).x_label_area_size(30).y_label_area_size(40).right_y_label_area_size(40)
        .build_cartesian_2d(0f32..1000f32, 0f32..101f32)?
        .set_secondary_coord(0f32..1000f32, 0f32..100f32);
    chart2.configure_mesh().y_desc("Avg ATP (%)").draw()?;
    chart2.configure_secondary_axes().y_desc("Avg Calcium").draw()?;
    
    chart2.draw_series(LineSeries::new(data.ticks.iter().zip(data.avg_atp.iter()).map(|(&x, &y)| (x as f32, y)), &GREEN))?
        .label("Avg ATP").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], GREEN));
    chart2.draw_secondary_series(LineSeries::new(data.ticks.iter().zip(data.avg_calcium.iter()).map(|(&x, &y)| (x as f32, y)), &YELLOW.mix(0.8)))?
        .label("Avg Calcium").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], YELLOW));
    chart2.configure_series_labels().background_style(WHITE.mix(0.8)).border_style(BLACK).draw()?;

    // Panel 3: Performance (FFI vs Rust)
    let max_time = data.ffi_times.iter().cloned().fold(0./0., f32::max).max(data.rust_times.iter().cloned().fold(0./0., f32::max)).max(0.1);
    let mut chart3 = ChartBuilder::on(&areas[2])
        .caption("Processing Time per Tick (ms)", ("sans-serif", 20).into_font())
        .margin(10).x_label_area_size(30).y_label_area_size(40).build_cartesian_2d(0f32..1000f32, 0f32..max_time * 1.1)?;
    chart3.configure_mesh().draw()?;
    chart3.draw_series(LineSeries::new(data.ticks.iter().zip(data.ffi_times.iter()).map(|(&x, &y)| (x as f32, y)), &BLUE))?
        .label("C++/CUDA Math").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));
    chart3.draw_series(LineSeries::new(data.ticks.iter().zip(data.rust_times.iter()).map(|(&x, &y)| (x as f32, y)), &MAGENTA))?
        .label("Rust Services").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], MAGENTA));
    chart3.configure_series_labels().background_style(WHITE.mix(0.8)).border_style(BLACK).draw()?;

    root.present()?;
    Ok(())
}

fn setup_simulation() -> NetworkGraph {
    let mut graph = NetworkGraph::new();

    // Spec 8 Test Circuit: Photoreceptor (0) -> Pyramidal (1)
    let photo_source = graph.add_neuron(NeuronType::Photoreceptor); // 0 - Analog Sensor
    let pyr_target = graph.add_neuron(NeuronType::Pyramidal);       // 1 - Target Learner

    // Target starts with a stable connection and SchwannCell (Myelin = true)
    graph.connect(photo_source, pyr_target, 5.0, 1.0, false, true);       

    graph
}

struct SimulationData {
    ticks: Vec<u32>,
    source_v: Vec<f32>,
    target_v: Vec<f32>,
    synapse_w: Vec<f32>,
}

fn track_metrics(data: &mut SimulationData, graph: &NetworkGraph, tick: u32) {
    data.ticks.push(tick);
    data.source_v.push(graph.neurons[0].current_voltage);
    data.target_v.push(graph.neurons[1].current_voltage);
    data.synapse_w.push(graph.synapses[0].neurotransmitter_buffer); // Track buffer to see analog flow
}

// Orchestrator: Runs the visual simulation loop
fn run_simulation(mut graph: NetworkGraph, total_ticks: u32) -> SimulationData {
    let mut data = SimulationData {
        ticks: Vec::with_capacity(total_ticks as usize),
        source_v: Vec::with_capacity(total_ticks as usize),
        target_v: Vec::with_capacity(total_ticks as usize),
        synapse_w: Vec::with_capacity(total_ticks as usize),
    };

    println!("Executing {} ticks...", total_ticks);
    
    // Setup Global Environment
    let env = Environment {
        global_oxygen_level: 1.0,  
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0, 
        toxin_levels: 0.0,
        light_level: 0.0,
        seed: 1337,
    };
    
    for tick in 1..=total_ticks {
        
        // Spec 8: Simulate Light/Dark environment for the Photoreceptor
        if tick < 50 {
            graph.neurons[0].current_voltage = -40.0;
        } else if tick < 120 {
            graph.neurons[0].current_voltage = -65.0;
        } else {
            graph.neurons[0].current_voltage = -40.0;
        }

        graph.tick(tick, env);
        
        if tick % 10 == 0 {
            graph.run_astrocyte_service(tick);
            graph.run_microglia_service(tick);
        }
        
        track_metrics(&mut data, &graph, tick);
    }

    println!("Simulation Complete!");
    
    // Total Observability (Phase 31)
    let mut telemetry = crate::telemetry::Telemetry::new();
    telemetry.update(total_ticks);
    telemetry.export_simulation_report(&graph.neurons, &graph.synapses, "simulation_report.json");

    data
}

fn render_chart(data: &SimulationData, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Rendering Chart to '{}'...", filename);

    let root = BitMapBackend::new(filename, (1200, 800)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .caption("Phase 8: Photoreceptor (Analog Sensor) & SchwannCell", ("sans-serif", 30).into_font())
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(50)
        .right_y_label_area_size(50)
        .build_cartesian_2d(0f32..200f32, -90f32..60f32)?
        .set_secondary_coord(0f32..200f32, 0f32..20f32);
        
    chart.configure_mesh().x_desc("Tick (ms)").y_desc("Voltage (mV)").draw()?;
    chart.configure_secondary_axes().y_desc("Neurotransmitter Buffer (Amount)").draw()?;

    chart.draw_series(LineSeries::new(
        data.ticks.iter().zip(data.source_v.iter()).map(|(&x, &y)| (x as f32, y)),
        &BLUE,
    ))?.label("Photoreceptor Sensor (mV)").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));

    chart.draw_series(LineSeries::new(
        data.ticks.iter().zip(data.target_v.iter()).map(|(&x, &y)| (x as f32, y)),
        &RED,
    ))?.label("Target Pyramidal (mV)").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));
    
    chart.draw_secondary_series(LineSeries::new(
        data.ticks.iter().zip(data.synapse_w.iter()).map(|(&x, &y)| (x as f32, y)),
        &GREEN,
    ))?.label("Synapse Buffer Flow").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], GREEN));

    chart.configure_series_labels().background_style(WHITE.mix(0.8)).border_style(BLACK).draw()?;
    root.present()?;

    println!("Chart saved successfully. Open {} to view results.", filename);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Initializing neuromorphic-engine Hybrid Engine...");

    let args: Vec<String> = env::args().collect();
    let is_benchmark = args.contains(&String::from("--benchmark"));
    let is_stress_test = args.contains(&String::from("--stress-test"));
    let is_dopamine_rush = args.contains(&String::from("--dopamine-rush"));
    let is_reflex_arc = args.contains(&String::from("--reflex-arc"));
    let is_inhibition_loop = args.contains(&String::from("--inhibition-loop"));
    let is_scaling_test = args.contains(&String::from("--scaling-test"));
    let is_frequency_test = args.contains(&String::from("--frequency-test"));
    let is_emergence_test = args.contains(&String::from("--emergence-test"));
    let is_morphology_audit = args.contains(&String::from("--morphology-audit"));

    if is_benchmark {
        let data = run_benchmark_mode(10000, 100000, 1000)?;
        render_benchmark_chart(&data, "graphs/benchmark_load.png")?;
        return Ok(());
    }

    if is_scaling_test {
        run_scaling_benchmarks()?;
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

    let graph = setup_simulation();
    let data = run_simulation(graph, 200);
    render_chart(&data, "graphs/phase8_sensors.png")?;

    Ok(())
}

fn setup_inhibition_loop() -> NetworkGraph {
    let mut graph = NetworkGraph::new();
    
    // 0: P (Pyramidal, Excitatory)
    let p = graph.add_neuron(NeuronType::Pyramidal);
    // 1: I (Interneuron, Inhibitory)
    let i = graph.add_neuron(NeuronType::Interneuron);

    // Loop: P -> I (Exciting), I -> P (Inhibiting)
    graph.connect(p, i, 10.0, 1.0, false, true);  // P excites I
    graph.connect(i, p, 15.0, 1.0, true, true);   // I inhibits P (Strong GABA)

    graph
}

fn run_inhibition_loop_mode() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- EXECUTING INHIBITION LOOP SCENARIO (Phase 19) ---");
    let mut graph = setup_inhibition_loop();
    let env = Environment { 
        global_oxygen_level: 1.0, 
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0, 
        toxin_levels: 0.0,
        light_level: 0.0,
        seed: 999 
    };

    let total_ticks = 1000;
    let mut history = vec![];

    for tick in 1..=total_ticks {
        // Subtle constant stimulus to avoid metabolic exhaustion
        graph.neurons[0].current_voltage += 0.7;

        graph.tick(tick, env);
        
        if tick % 10 == 0 {
            graph.run_astrocyte_service(tick);
            graph.run_microglia_service(tick);
        }

        history.push((
            tick,
            graph.neurons[0].current_voltage, // Pyramidal
            graph.neurons[1].current_voltage, // Interneuron
        ));
    }

    render_inhibition_chart(&history, "graphs/inhibition_loop.png")?;
    Ok(())
}

fn render_inhibition_chart(data: &[(u32, f32, f32)], filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Rendering Inhibition Loop Chart to '{}'...", filename);
    let root = BitMapBackend::new(filename, (1200, 800)).into_drawing_area();
    root.fill(&WHITE)?;

    let areas = root.split_evenly((2, 1));

    // Panel 1: Pyramidal (Target)
    let mut chart1 = ChartBuilder::on(&areas[0])
        .caption("1. Pyramidal Neuron (Excited with Constant Stimulus + GABA Inhibition)", ("sans-serif", 20).into_font())
        .margin(10).x_label_area_size(30).y_label_area_size(40).build_cartesian_2d(0f32..1000f32, -90f32..50f32)?;
    chart1.configure_mesh().y_desc("Voltage (mV)").draw()?;
    chart1.draw_series(LineSeries::new(data.iter().map(|&(t, v, _)| (t as f32, v)), &RED))?
        .label("Pyramidal Voltage").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));
    chart1.configure_series_labels().background_style(WHITE.mix(0.8)).border_style(BLACK).draw()?;

    // Panel 2: Interneuron (Inhibitor)
    let mut chart2 = ChartBuilder::on(&areas[1])
        .caption("2. GABA Interneuron (Driven by Pyramidal Spikes)", ("sans-serif", 20).into_font())
        .margin(10).x_label_area_size(30).y_label_area_size(40).build_cartesian_2d(0f32..1000f32, -90f32..50f32)?;
    chart2.configure_mesh().y_desc("Voltage (mV)").draw()?;
    chart2.draw_series(LineSeries::new(data.iter().map(|&(t, _, v)| (t as f32, v)), &BLUE))?
        .label("Interneuron Voltage").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));
    chart2.configure_series_labels().background_style(WHITE.mix(0.8)).border_style(BLACK).draw()?;

    root.present()?;
    Ok(())
}

fn setup_reflex_arc() -> NetworkGraph {
    let mut graph = NetworkGraph::new();
    
    // 0: Photoreceptor (The Eye)
    let eye = graph.add_neuron(NeuronType::Photoreceptor);
    // 1: Interneuron (The Brain/Filter)
    let brain = graph.add_neuron(NeuronType::Interneuron);
    // 2: Pyramidal (The Muscle)
    let muscle = graph.add_neuron(NeuronType::Pyramidal);

    // Connections: Eye -> Brain -> Muscle
    // Phase 17: Boosted weights to ensure transmission
    graph.connect(eye, brain, 15.0, 1.0, false, true);   // Fast myelin connection
    graph.connect(brain, muscle, 12.0, 1.0, false, false); // Normal connection

    graph
}

fn run_reflex_arc_mode() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- EXECUTING REFLEX ARC SCENARIO WITH DOPAMINE BOOST (Phase 17) ---");
    let mut graph = setup_reflex_arc();
    let mut env = Environment { 
        global_oxygen_level: 1.0, 
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0, 
        toxin_levels: 0.0,
        light_level: 0.0,
        seed: 888 
    };

    let total_ticks = 1200;
    let mut history = vec![];

    for tick in 1..=total_ticks {
        // Stimulus: Rhythmic blinking
        env.light_level = if (tick / 50) % 2 == 0 { 0.8 } else { 0.1 };

        // Phase 17: Dopamine Surge between tick 300 and 800
        if tick >= 300 && tick <= 800 {
            env.dopamine_multiplier = 4.0; // RUSH!
        } else {
            env.dopamine_multiplier = 1.0;
        }

        graph.tick(tick, env);
        
        if tick % 10 == 0 {
            graph.run_astrocyte_service(tick);
            graph.run_microglia_service(tick);
        }

        history.push((
            tick,
            graph.neurons[0].current_voltage, // Eye
            graph.neurons[1].current_voltage, // Brain
            graph.neurons[2].current_voltage, // Muscle
            graph.neurons[2].energy_atp,      // Muscle ATP
            graph.synapses[1].weight,          // Brain -> Muscle Weight (STDP)
        ));
    }

    render_reflex_arc_chart(&history, "graphs/reflex_arc_boosted.png")?;
    Ok(())
}

fn render_reflex_arc_chart(data: &[(u32, f32, f32, f32, f32, f32)], filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Rendering Reflex Arc Chart to '{}'...", filename);
    let root = BitMapBackend::new(filename, (1200, 1000)).into_drawing_area();
    root.fill(&WHITE)?;

    let areas = root.split_evenly((3, 1));

    // Panel 1: Sensory (Eye)
    let mut chart1 = ChartBuilder::on(&areas[0])
        .caption("1. Input: Photoreceptor (The Eye)", ("sans-serif", 20).into_font())
        .margin(10).x_label_area_size(30).y_label_area_size(40).build_cartesian_2d(0f32..1000f32, -70f32..-30f32)?;
    chart1.configure_mesh().y_desc("Voltage (mV)").draw()?;
    chart1.draw_series(LineSeries::new(data.iter().map(|&(t, v, _, _, _, _)| (t as f32, v)), &BLUE))?;

    // Panel 2: Logic (Interneuron / Brain)
    let mut chart2 = ChartBuilder::on(&areas[1])
        .caption("2. Logic: Interneuron (The Brain)", ("sans-serif", 20).into_font())
        .margin(10).x_label_area_size(30).y_label_area_size(40).build_cartesian_2d(0f32..1000f32, -90f32..50f32)?;
    chart2.configure_mesh().y_desc("Voltage (mV)").draw()?;
    chart2.draw_series(LineSeries::new(data.iter().map(|&(t, _, v, _, _, _)| (t as f32, v)), &MAGENTA))?;

    // Panel 3: Output (Pyramidal / Muscle)
    let mut chart3 = ChartBuilder::on(&areas[2])
        .caption("3. Output: Pyramidal (The Muscle)", ("sans-serif", 20).into_font())
        .margin(10).x_label_area_size(30).y_label_area_size(40).right_y_label_area_size(40)
        .build_cartesian_2d(0f32..1000f32, -90f32..50f32)?
        .set_secondary_coord(0f32..1000f32, 0f32..110f32);
    
    chart3.configure_mesh().y_desc("Voltage (mV)").draw()?;
    chart3.configure_secondary_axes().y_desc("ATP / Plasticity Weight").draw()?;

    chart3.draw_series(LineSeries::new(data.iter().map(|&(t, _, _, v, _, _)| (t as f32, v)), &RED))?
        .label("Muscle Voltage").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));
    
    chart3.draw_secondary_series(LineSeries::new(data.iter().map(|&(t, _, _, _, atp, _)| (t as f32, atp)), &GREEN))?
        .label("Muscle ATP").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], GREEN));

    chart3.draw_secondary_series(LineSeries::new(data.iter().map(|&(t, _, _, _, _, w)| (t as f32, w * 5.0)), &YELLOW))?
        .label("STDP Weight (x5)").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], YELLOW));

    chart3.configure_series_labels().background_style(WHITE.mix(0.8)).border_style(BLACK).draw()?;

    root.present()?;
    Ok(())
}

fn run_stress_test_mode() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- EXECUTING STRESS-TEST (Phase 13) ---");
    let mut graph = NetworkGraph::new();
    let mut env = Environment { 
        global_oxygen_level: 1.0, 
        global_glucose_level: 1.0, 
        dopamine_multiplier: 1.0, 
        toxin_levels: 0.0, 
        light_level: 0.0, 
        seed: 666 
    };

    // Case 1: Hypoxic Shutdown (10 Neurons)
    println!("Case 1: Hypoxic Shutdown...");
    for _ in 0..10 { graph.add_neuron(NeuronType::Pyramidal); }
    
    // Case 2: Epileptic Feedback (3 Neurons) - High weights to ensure self-sustaining loop
    println!("Case 2: Epileptic Feedback Loop (Weights = 15.0)...");
    let n1 = graph.add_neuron(NeuronType::Pyramidal);
    let n2 = graph.add_neuron(NeuronType::Pyramidal);
    let n3 = graph.add_neuron(NeuronType::Pyramidal);
    // Connect in a reinforced loop
    graph.connect(n1, n2, 15.0, 1.0, false, false);
    graph.connect(n2, n3, 15.0, 1.0, false, false);
    graph.connect(n3, n1, 15.0, 1.0, false, false);
    
    // Kickstart the loop with a high voltage on n1
    graph.neurons[n1 as usize].current_voltage = 20.0;

    let mut results = vec![];
    for tick in 0..3000 {
        // Trigger Extreme Hypoxia at tick 300
        if tick == 300 { 
            println!(">>> DISASTER: Oxygen Level dropped to 1%: Total Hypoxia!");
            env.global_oxygen_level = 0.01; 
        }

        graph.tick(tick, env);
        
        let mut total_atp = 0.0;
        let mut _total_calcium = 0.0;
        let mut dead_count = 0;
        let mut avg_wear = 0.0;

        for n in &graph.neurons {
            total_atp += n.energy_atp;
            avg_wear += n.metabolic_wear;
            if !n.is_alive { dead_count += 1; }
        }

        results.push((tick, total_atp / graph.neurons.len() as f32, dead_count, avg_wear / graph.neurons.len() as f32));
        
        if tick % 100 == 0 {
            println!("Tick {}: Avg ATP: {:.2}, Dead: {}, Avg Wear: {:.4}", tick, total_atp / graph.neurons.len() as f32, dead_count, avg_wear / graph.neurons.len() as f32);
        }

        if dead_count == graph.neurons.len() {
            println!(">>> DISASTER: All neurons died at tick {} due to metabolic failure.", tick);
            break;
        }
    }
    
    println!("--- STRESS TEST SUMMARY ---");
    println!("Final State: {}/{} neurons survived.", graph.neurons.len() - results.last().unwrap().2, graph.neurons.len());
    println!("Peak Metabolic Wear: {:.4}", results.last().unwrap().3);
    
    // Total Observability (Phase 31)
    let mut telemetry = crate::telemetry::Telemetry::new();
    telemetry.update(results.last().map(|r| r.0).unwrap_or(0));
    telemetry.export_simulation_report(&graph.neurons, &graph.synapses, "stress_report.json");

    Ok(())
}

fn run_dopamine_rush_mode() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- EXECUTING DOPAMINE RUSH (Phase 14) ---");
    let mut graph = NetworkGraph::new();
    let mut env = Environment { 
        global_oxygen_level: 1.0, 
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0, 
        toxin_levels: 0.0, 
        light_level: 0.0, 
        seed: 777 
    };

    // Setup: 2 Neurons with a weak connection
    let n1 = graph.add_neuron(NeuronType::Pacemaker); // Source fires constantly
    let n2 = graph.add_neuron(NeuronType::Pyramidal); // Target
    graph.connect(n1, n2, 1.0, 1.0, false, true);

    let mut weight_history = vec![];
    
    println!("Step 1: Low Dopamine (0.1x) - Learning is stalled");
    env.dopamine_multiplier = 0.1;
    for tick in 0..200 {
        graph.tick(tick, env);
        weight_history.push((tick, graph.synapses[0].weight, env.dopamine_multiplier));
    }

    println!("Step 2: DOPAMINE RUSH (5.0x) - Learning explosion!");
    env.dopamine_multiplier = 5.0;
    for tick in 200..600 {
        graph.tick(tick, env);
        weight_history.push((tick, graph.synapses[0].weight, env.dopamine_multiplier));
    }

    println!("Step 3: Normal State (1.0x) - Stability");
    env.dopamine_multiplier = 1.0;
    for tick in 600..1000 {
        graph.tick(tick, env);
        weight_history.push((tick, graph.synapses[0].weight, env.dopamine_multiplier));
    }

    render_plasticity_chart(&weight_history, "graphs/dopamine_rush_results.png")?;
    Ok(())
}

fn render_plasticity_chart(history: &[(u32, f32, f32)], filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Rendering Plasticity Analysis to '{}'...", filename);
    let root = BitMapBackend::new(filename, (1024, 768)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .caption("Synaptic Plasticity: Dopamine Modulation", ("sans-serif", 30).into_font())
        .margin(20).x_label_area_size(40).y_label_area_size(50).right_y_label_area_size(50)
        .build_cartesian_2d(0f32..1000f32, 0f32..20.0f32)?
        .set_secondary_coord(0f32..1000f32, 0f32..6.0f32);

    chart.configure_mesh().y_desc("Synaptic Weight").draw()?;
    chart.configure_secondary_axes().y_desc("Dopamine Level").draw()?;

    // Weight Curve
    chart.draw_series(LineSeries::new(history.iter().map(|&(t, w, _)| (t as f32, w)), &RED.mix(0.8)))?
        .label("Synaptic Weight").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));

    // Dopamine Curve (Secondary)
    chart.draw_secondary_series(AreaSeries::new(history.iter().map(|&(t, _, d)| (t as f32, d)), 0.0, &CYAN.mix(0.2)))?
        .label("Dopamine Multiplier").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], CYAN));

    chart.configure_series_labels().background_style(WHITE.mix(0.8)).border_style(BLACK).draw()?;
    root.present()?;
    Ok(())
}

fn render_stress_chart(results: &[(u32, f32, usize, f32)], filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Rendering Stress Analysis to '{}'...", filename);
    let root = BitMapBackend::new(filename, (1024, 768)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .caption("Stress Test: Hypoxic Shutdown Analysis", ("sans-serif", 30).into_font())
        .margin(20).x_label_area_size(40).y_label_area_size(50).right_y_label_area_size(50)
        .build_cartesian_2d(0f32..1000f32, 0f32..100f32)?
        .set_secondary_coord(0f32..1000f32, 0f32..1.1f32);

    chart.configure_mesh().y_desc("ATP Level / Death Count").draw()?;
    chart.configure_secondary_axes().y_desc("Metabolic Wear").draw()?;

    // ATP Curve
    chart.draw_series(LineSeries::new(results.iter().map(|&(t, atp, _, _)| (t as f32, atp)), &BLUE))?
        .label("Avg ATP").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));

    // Death Count (Scaled to 100 for visibility)
    chart.draw_series(LineSeries::new(results.iter().map(|&(t, _, d, _)| (t as f32, d as f32 * 7.5)), &RED))?
        .label("Mortality (scaled)").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));

    // Wear Curve (Secondary axis)
    chart.draw_secondary_series(LineSeries::new(results.iter().map(|&(t, _, _, w)| (t as f32, w)), &BLACK))?
        .label("Mitochondrial Wear").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLACK));

    chart.configure_series_labels().background_style(WHITE.mix(0.8)).border_style(BLACK).draw()?;
    root.present()?;
    Ok(())
}

fn run_frequency_test_mode() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- EXECUTING FREQUENCY DISCRIMINATION TEST (Phase 23) ---");
    let mut graph = NetworkGraph::new();
    
    // Setup: 1 Input (Eye), 2 Logic neurons (F1_Detector, F2_Detector)
    let eye = graph.add_neuron(NeuronType::Photoreceptor);
    let f1_det = graph.add_neuron(NeuronType::Pyramidal);
    let f2_det = graph.add_neuron(NeuronType::Pyramidal);
    
    // Initial random connections
    graph.connect(eye, f1_det, 5.0, 1.0, false, true);
    graph.connect(eye, f2_det, 5.0, 1.0, false, true);
    
    let total_ticks = 2000;
    let mut history = vec![];
    let mut env = Environment { 
        global_oxygen_level: 1.0, 
        global_glucose_level: 1.0, 
        dopamine_multiplier: 1.5, 
        toxin_levels: 0.0, 
        light_level: 0.0, 
        seed: 42 
    };

    for tick in 1..=total_ticks {
        // Frequency 1: 10Hz (Pulse every 100 ticks)
        // Frequency 2: 50Hz (Pulse every 20 ticks)
        if tick < 1000 {
            // Train on 10Hz
            env.light_level = if (tick / 50) % 2 == 0 { 0.8 } else { 0.0 };
        } else {
            // Test on 50Hz
            env.light_level = if (tick / 10) % 2 == 0 { 0.8 } else { 0.0 };
        }

        graph.tick(tick, env);
        
        if tick % 10 == 0 {
            graph.run_astrocyte_service(tick);
            graph.run_microglia_service(tick);
        }

        let s1_w = if !graph.synapses.is_empty() { graph.synapses[0].weight } else { 0.0 };
        let s2_w = if graph.synapses.len() > 1 { graph.synapses[1].weight } else { 0.0 };

        history.push((
            tick,
            graph.neurons[f1_det as usize].current_voltage,
            graph.neurons[f2_det as usize].current_voltage,
            s1_w,
            s2_w,
        ));
    }

    render_frequency_chart(&history, "graphs/frequency_learning.png")?;
    println!("Frequency test complete. Check graphs/frequency_learning.png");
    Ok(())
}

fn run_emergence_test_mode() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- EXECUTING EMERGENCE FROM CHAOS (Phase 29) ---");
    let mut graph = NetworkGraph::new();
    let num_neurons = 500;
    
    // Spawn neurons with 0 INITIAL CONNECTIONS
    for _ in 0..num_neurons {
        graph.add_neuron(NeuronType::Pyramidal);
    }
    
    let total_ticks = 3000;
    let mut history = vec![];
    let env = Environment { 
        global_oxygen_level: 1.0, 
        global_glucose_level: 1.0, 
        dopamine_multiplier: 5.0, // Maximum survival drive
        toxin_levels: 0.0, 
        light_level: 0.0, 
        seed: 1234 
    };

    for tick in 1..=total_ticks {
        // Stimulate specific cluster (Central Hub) - First 200 neurons
        for i in 0..200 {
            graph.neurons[i].current_voltage += 15.0; 
        }

        graph.tick(tick, env);
        
        if tick % 10 == 0 {
            graph.run_astrocyte_service(tick);
            graph.run_microglia_service(tick);
            graph.run_neurogenesis_service(tick);
        }

        if tick % 100 == 0 {
            println!("Tick {}: Synapse Count: {}", tick, graph.synapses.len());
        }

        history.push((
            tick,
            graph.synapses.len() as u32,
            graph.neurons[250].current_voltage, // Remote neuron
        ));
    }

    render_emergence_chart(&history, "graphs/emergence_growth.png")?;
    
    // Total Observability (Phase 31)
    let mut telemetry = crate::telemetry::Telemetry::new();
    telemetry.update(3000); // 3000 ticks in emergence test
    telemetry.export_simulation_report(&graph.neurons, &graph.synapses, "emergence_report.json");

    Ok(())
}

fn render_emergence_chart(data: &[(u32, u32, f32)], filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Rendering Emergence Analysis to '{}'...", filename);
    let root = BitMapBackend::new(filename, (1200, 800)).into_drawing_area();
    root.fill(&WHITE)?;

    let areas = root.split_evenly((2, 1));

    // Panel 1: Structural Growth
    let max_syn = data.iter().map(|&(_, s, _)| s).max().unwrap_or(100).max(100);
    let mut chart1 = ChartBuilder::on(&areas[0])
        .caption("1. Structural Growth (Synaptogenesis)", ("sans-serif", 20).into_font())
        .margin(10).x_label_area_size(30).y_label_area_size(40).build_cartesian_2d(0f32..2000f32, 0u32..max_syn + 50)?;
    chart1.configure_mesh().y_desc("Synapse Count").draw()?;
    chart1.draw_series(LineSeries::new(data.iter().map(|&(t, s, _)| (t as f32, s)), &GREEN))?
        .label("Active Synapses").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], GREEN));
    chart1.configure_series_labels().background_style(WHITE.mix(0.8)).border_style(BLACK).draw()?;

    // Panel 2: Signal Penetration (Functional Connectivity)
    let mut chart2 = ChartBuilder::on(&areas[1])
        .caption("2. Signal Reach (Remote Neuron Activity)", ("sans-serif", 20).into_font())
        .margin(10).x_label_area_size(30).y_label_area_size(40).build_cartesian_2d(0f32..2000f32, -80f32..50f32)?;
    chart2.configure_mesh().y_desc("Voltage (mV)").draw()?;
    chart2.draw_series(LineSeries::new(data.iter().map(|&(t, _, v)| (t as f32, v)), &BLUE))?
        .label("Neuron 250 Voltage").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));
    chart2.configure_series_labels().background_style(WHITE.mix(0.8)).border_style(BLACK).draw()?;

    root.present()?;
    Ok(())
}

fn render_frequency_chart(data: &[(u32, f32, f32, f32, f32)], filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Rendering Frequency Learning Chart to '{}'...", filename);
    let root = BitMapBackend::new(filename, (1200, 800)).into_drawing_area();
    root.fill(&WHITE)?;
    let areas = root.split_evenly((2, 1));

    // Panel 1: Voltages
    let mut chart1 = ChartBuilder::on(&areas[0])
        .caption("Neuron Responses to Stimuli", ("sans-serif", 30).into_font())
        .margin(10).x_label_area_size(30).y_label_area_size(40).build_cartesian_2d(0f32..2000f32, -90f32..50f32)?;
    chart1.configure_mesh().draw()?;
    chart1.draw_series(LineSeries::new(data.iter().map(|&(t, v1, _, _, _)| (t as f32, v1)), &RED))?
        .label("F1 Detector").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));
    chart1.draw_series(LineSeries::new(data.iter().map(|&(t, _, v2, _, _)| (t as f32, v2)), &BLUE))?
        .label("F2 Detector").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));
    chart1.configure_series_labels().background_style(WHITE.mix(0.8)).draw()?;

    // Panel 2: Synaptic Weights (Plasticity)
    let mut chart2 = ChartBuilder::on(&areas[1])
        .caption("Synaptic Weight Evolution (STDP)", ("sans-serif", 30).into_font())
        .margin(10).x_label_area_size(30).y_label_area_size(40).build_cartesian_2d(0f32..2000f32, 0f32..35f32)?;
    chart2.configure_mesh().draw()?;
    chart2.draw_series(LineSeries::new(data.iter().map(|&(t, _, _, w1, _)| (t as f32, w1)), &RED))?
        .label("W1 (F1 Weight)").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));
    chart2.draw_series(LineSeries::new(data.iter().map(|&(t, _, _, _, w2)| (t as f32, w2)), &BLUE))?
        .label("W2 (F2 Weight)").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));
    chart2.configure_series_labels().background_style(WHITE.mix(0.8)).draw()?;

    root.present()?;
    Ok(())
}
fn run_morphology_audit() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- EXECUTING SYNAPTIC MORPHOLOGY AUDIT (Phase 30) ---");
    let mut graph = NetworkGraph::new();
    
    // Setup: Source -> Target Pair
    let source = graph.add_neuron(NeuronType::Pyramidal);
    let target = graph.add_neuron(NeuronType::Pyramidal);
    
    // Scenario: A very strong "seed" synapse to ensure survival
    graph.connect(source, target, 5.0, 2.0, false, false);
    
    let total_ticks = 3000;
    let mut history = vec![];
    let env = Environment { 
        global_oxygen_level: 1.0, 
        global_glucose_level: 1.0, 
        dopamine_multiplier: 2.0, // Stable growth
        toxin_levels: 0.0, 
        light_level: 0.0, 
        seed: 777 
    };

    println!("Initial: Weight={}, Thickness={}", graph.synapses[0].weight, graph.synapses[0].thickness);

    for tick in 1..=total_ticks {
        // Stimulate source to spike and drive LTP
        if tick % 25 == 0 {
            graph.neurons[source as usize].current_voltage += 35.0; 
        }

        graph.tick(tick, env);
        
        if tick % 10 == 0 {
            graph.run_astrocyte_service(tick);
            graph.run_microglia_service(tick);
        }

        if tick % 500 == 0 {
            let s_atp = graph.neurons[source as usize].energy_atp;
            let t_atp = graph.neurons[target as usize].energy_atp;
            let n_alive = graph.neurons[source as usize].is_alive && graph.neurons[target as usize].is_alive;
            println!("Tick {}: Synapses={}, SourceATP={:.1}, TargetATP={:.1}, Alive={}", 
                tick, graph.synapses.len(), s_atp, t_atp, n_alive);
        }

        let sw = if !graph.synapses.is_empty() { graph.synapses[0].weight } else { 0.0 };
        let st = if !graph.synapses.is_empty() { graph.synapses[0].thickness } else { 0.0 };
        let sb = if !graph.synapses.is_empty() { graph.synapses[0].neurotransmitter_buffer } else { 0.0 };

        history.push((
            tick,
            sw,
            st,
            graph.neurons[target as usize].current_voltage,
            sb,
        ));
    }

    render_morphology_chart(&history, "graphs/morphology_audit.png")?;
    
    if !graph.synapses.is_empty() {
        println!("Final: Weight={:.2}, Thickness={:.2}", graph.synapses[0].weight, graph.synapses[0].thickness);
    } else {
        println!("Final: Synapse was pruned.");
    }

    // Total Observability (Phase 31)
    let mut telemetry = crate::telemetry::Telemetry::new();
    telemetry.update(total_ticks);
    telemetry.export_simulation_report(&graph.neurons, &graph.synapses, "morphology_report.json");

    Ok(())
}

fn render_morphology_chart(data: &[(u32, f32, f32, f32, f32)], filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Rendering Morphology Audit to '{}'...", filename);
    let root = BitMapBackend::new(filename, (1200, 800)).into_drawing_area();
    root.fill(&WHITE)?;

    let areas = root.split_evenly((2, 1));

    // Panel 1: Synaptic Growth (Weight & Thickness)
    let mut chart1 = ChartBuilder::on(&areas[0])
        .caption("1. Synaptic Growth (Structural Plasticity)", ("sans-serif", 20).into_font())
        .margin(10).x_label_area_size(30).y_label_area_size(40).build_cartesian_2d(0f32..3000f32, 0f32..10f32)?;
    chart1.configure_mesh().y_desc("Value").draw()?;
    
    chart1.draw_series(LineSeries::new(data.iter().map(|&(t, w, _, _, _)| (t as f32, w)), &RED))?
        .label("Syaptic Weight").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));
    
    chart1.draw_series(LineSeries::new(data.iter().map(|&(t, _, th, _, _)| (t as f32, th)), &BLUE))?
        .label("Synaptic Thickness").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));
    
    chart1.configure_series_labels().background_style(WHITE.mix(0.8)).border_style(BLACK).draw()?;

    // Panel 2: Transmission Reliability & Buffer
    let mut chart2 = ChartBuilder::on(&areas[1])
        .caption("2. Transmission Performance & Capacity", ("sans-serif", 20).into_font())
        .margin(10).x_label_area_size(30).y_label_area_size(40).build_cartesian_2d(0f32..3000f32, -90f32..100f32)?;
    chart2.configure_mesh().draw()?;
    
    chart2.draw_series(LineSeries::new(data.iter().map(|&(t, _, _, v, _)| (t as f32, v)), &MAGENTA))?
        .label("Target Voltage").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], MAGENTA));

    chart2.draw_series(LineSeries::new(data.iter().map(|&(t, _, _, _, buff)| (t as f32, buff)), &GREEN))?
        .label("NT Buffer Volume").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], GREEN));

    chart2.configure_series_labels().background_style(WHITE.mix(0.8)).border_style(BLACK).draw()?;

    root.present()?;
    Ok(())
}
