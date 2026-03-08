// src/main.rs
mod ffi;
mod graph;

use ffi::{NeuronType, Environment};
use graph::NetworkGraph;
use plotters::prelude::*;
use std::env;
use std::time::Instant;
use rand::prelude::*;

fn setup_benchmark(neurons_count: usize, synapses_count: usize) -> NetworkGraph {
    let mut graph = NetworkGraph::new();
    let mut rng = rand::rng();

    println!("Generating Benchmark Network: {} neurons, {} synapses...", neurons_count, synapses_count);

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

    // Creating random synapses
    for _ in 0..synapses_count {
        let source = rng.random_range(0..(neurons_count as u32));
        let target = rng.random_range(0..(neurons_count as u32));
        let weight = rng.random_range(0.5..5.0);
        let is_inhib = rng.random_bool(0.2); // 20% inhibitory
        let is_myelin = rng.random_bool(0.1); // 10% myelinated

        graph.connect(source, target, weight, is_inhib, is_myelin);
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
        dopamine_multiplier: 1.0, 
        toxin_levels: 0.0,
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

    for tick in 1..=total_ticks {
        let num_neurons = graph.neurons.len() as i32;

        let ffi_start = Instant::now();
        graph.tick(tick, env);
        let ffi_elapsed = ffi_start.elapsed();
        total_ffi_time += ffi_elapsed;
        
        // Metrics collection for this tick
        let mut spikes = 0;
        let mut atp_sum = 0.0;
        let mut calcium_sum = 0.0;
        for n in &graph.neurons {
            if n.current_voltage >= 39.0 { spikes += 1; }
            atp_sum += n.energy_atp;
            calcium_sum += n.calcium_level;
        }

        let mut tick_rust_time = std::time::Duration::new(0, 0);
        if tick % 10 == 0 {
            let rust_start = Instant::now();
            graph.run_astrocyte_service(tick);
            graph.run_microglia_service(tick);
            tick_rust_time = rust_start.elapsed();
            total_rust_time += tick_rust_time;
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

    Ok(data)
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
    graph.connect(photo_source, pyr_target, 5.0, false, true);       

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
        dopamine_multiplier: 1.0, 
        toxin_levels: 0.0,
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

    if is_benchmark {
        let data = run_benchmark_mode(10000, 100000, 1000)?;
        render_benchmark_chart(&data, "graphs/benchmark_load.png")?;
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

    let graph = setup_simulation();
    let data = run_simulation(graph, 200);
    render_chart(&data, "graphs/phase8_sensors.png")?;

    Ok(())
}

fn run_stress_test_mode() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- EXECUTING STRESS-TEST (Phase 13) ---");
    let mut graph = NetworkGraph::new();
    let mut env = Environment { global_oxygen_level: 1.0, dopamine_multiplier: 1.0, toxin_levels: 0.0 };

    // Case 1: Hypoxic Shutdown (10 Neurons)
    println!("Case 1: Hypoxic Shutdown...");
    for _ in 0..10 { graph.add_neuron(NeuronType::Pyramidal); }
    
    // Case 2: Epileptic Feedback (3 Neurons) - High weights to ensure self-sustaining loop
    println!("Case 2: Epileptic Feedback Loop (Weights = 15.0)...");
    let n1 = graph.add_neuron(NeuronType::Pyramidal);
    let n2 = graph.add_neuron(NeuronType::Pyramidal);
    let n3 = graph.add_neuron(NeuronType::Pyramidal);
    // Connect in a reinforced loop
    graph.connect(n1, n2, 15.0, false, false);
    graph.connect(n2, n3, 15.0, false, false);
    graph.connect(n3, n1, 15.0, false, false);
    
    // Kickstart the loop with a high voltage on n1
    graph.neurons[n1 as usize].current_voltage = 20.0;

    let mut results = vec![];
    for tick in 0..1000 {
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
    
    // We can also render a special chart for this
    render_stress_chart(&results, "graphs/stress_test_results.png")?;
    
    Ok(())
}

fn run_dopamine_rush_mode() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- EXECUTING DOPAMINE RUSH (Phase 14) ---");
    let mut graph = NetworkGraph::new();
    let mut env = Environment { global_oxygen_level: 1.0, dopamine_multiplier: 1.0, toxin_levels: 0.0 };

    // Setup: 2 Neurons with a weak connection
    let n1 = graph.add_neuron(NeuronType::Pacemaker); // Source fires constantly
    let n2 = graph.add_neuron(NeuronType::Pyramidal); // Target
    graph.connect(n1, n2, 1.0, false, true);

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
