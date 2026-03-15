// src/visualization.rs
use plotters::prelude::*;
use crate::engine::constants::*;
use crate::runner::modes::{BenchmarkData, SimulationData};

type DrawingAreaBackend<'a> = DrawingArea<BitMapBackend<'a>, plotters::coord::Shift>;

// Helper: creates a standard f32/f32 chart panel with consistent styling.
pub fn init_chart_f32<'a>(
    area: &DrawingAreaBackend<'a>,
    caption: &str,
    x_range: std::ops::Range<f32>,
    y_range: std::ops::Range<f32>,
) -> Result<ChartContext<'a, BitMapBackend<'a>, Cartesian2d<plotters::coord::types::RangedCoordf32, plotters::coord::types::RangedCoordf32>>, Box<dyn std::error::Error>> {
    let mut chart = ChartBuilder::on(area)
        .caption(caption, ("sans-serif", CAPTION_FONT_SIZE as i32).into_font())
        .margin(CHART_MARGIN)
        .x_label_area_size(LABEL_AREA_SIZE)
        .y_label_area_size(LABEL_AREA_SIZE)
        .build_cartesian_2d(x_range, y_range)?;
    chart.configure_mesh().draw()?;
    Ok(chart)
}

// Helper: creates a standard f32/u32 chart panel (e.g. for spike counts) with consistent styling.
pub fn init_chart_u32<'a>(
    area: &DrawingAreaBackend<'a>,
    caption: &str,
    x_range: std::ops::Range<f32>,
    y_range: std::ops::Range<u32>,
) -> Result<ChartContext<'a, BitMapBackend<'a>, Cartesian2d<plotters::coord::types::RangedCoordf32, plotters::coord::types::RangedCoordu32>>, Box<dyn std::error::Error>> {
    let mut chart = ChartBuilder::on(area)
        .caption(caption, ("sans-serif", CAPTION_FONT_SIZE as i32).into_font())
        .margin(CHART_MARGIN)
        .x_label_area_size(LABEL_AREA_SIZE)
        .y_label_area_size(LABEL_AREA_SIZE)
        .build_cartesian_2d(x_range, y_range)?;
    chart.configure_mesh().draw()?;
    Ok(chart)
}

pub fn render_benchmark_chart(data: &BenchmarkData, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Rendering Benchmark Dashboard to '{}'...", filename);
    let root = BitMapBackend::new(filename, (CANVAS_WIDTH, CANVAS_HEIGHT_XL)).into_drawing_area();
    root.fill(&WHITE)?;

    let areas = root.split_evenly((3, 1));
    let x_range = 0f32..TICK_LIMIT_CHART;

    // Panel 1: Activity Histogram (Spikes)
    let max_spikes = data.total_spikes.iter().cloned().max().unwrap_or(1).max(100);
    let mut chart1 = init_chart_u32(&areas[0], "Network Activity (Total Spikes per Tick)", x_range.clone(), 0u32..max_spikes + 10)?;
    chart1.draw_series(data.ticks.iter().zip(data.total_spikes.iter()).map(|(&x, &y)| {
        Rectangle::new([(x as f32, 0), (x as f32 + 1.0, y)], RED.filled())
    }))?;

    // Panel 2: Biology Stats (Avg ATP & Calcium) - needs dual coord, so can't use helper
    let mut chart2 = ChartBuilder::on(&areas[1])
        .caption("Average Biology State", ("sans-serif", CAPTION_FONT_SIZE as i32).into_font())
        .margin(CHART_MARGIN)
        .x_label_area_size(LABEL_AREA_SIZE)
        .y_label_area_size(LABEL_AREA_SIZE)
        .right_y_label_area_size(LABEL_AREA_SIZE)
        .build_cartesian_2d(x_range.clone(), 0f32..101f32)?
        .set_secondary_coord(x_range.clone(), 0f32..100f32);
    chart2.configure_mesh().y_desc("Avg ATP (%)").draw()?;
    chart2.configure_secondary_axes().y_desc("Avg Calcium").draw()?;
    
    // Data is now scalar (total_atp_sum / samples_count). We can't plot it over time, so we just add a dummy line to keep legends if needed, or leave blank.
    let avg_atp_val = if data.samples_count > 0 { (data.total_atp_sum / data.samples_count as f64) as f32 } else { 0.0 };
    let avg_calc_val = if data.samples_count > 0 { (data.total_calcium_sum / data.samples_count as f64) as f32 } else { 0.0 };
    chart2.draw_series(LineSeries::new(vec![(0.0, avg_atp_val), (data.ticks.last().copied().unwrap_or(0) as f32, avg_atp_val)], &GREEN))?
        .label(format!("Avg ATP: {:.2}", avg_atp_val)).legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], GREEN));
    chart2.draw_secondary_series(LineSeries::new(vec![(0.0, avg_calc_val), (data.ticks.last().copied().unwrap_or(0) as f32, avg_calc_val)], &YELLOW.mix(0.8)))?
        .label(format!("Avg Calcium: {:.2}", avg_calc_val)).legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], YELLOW));
    chart2.configure_series_labels().background_style(WHITE.mix(0.8)).border_style(BLACK).draw()?;

    // Panel 3: Performance (FFI vs Rust)
    let max_time = data.ffi_times.iter().cloned().fold(f32::NAN, f32::max).max(data.rust_times.iter().cloned().fold(f32::NAN, f32::max)).max(0.1);
    let mut chart3 = init_chart_f32(&areas[2], "Processing Time per Tick (ms)", x_range.clone(), 0f32..max_time * 1.1)?;
    
    chart3.draw_series(LineSeries::new(data.ticks.iter().zip(data.ffi_times.iter()).map(|(&x, &y)| (x as f32, y)), &BLUE))?
        .label("CUDA (Math)").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));
    chart3.draw_series(LineSeries::new(data.ticks.iter().zip(data.rust_times.iter()).map(|(&x, &y)| (x as f32, y)), &MAGENTA.mix(0.6)))?
        .label("Rust (Services)").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], MAGENTA));
    chart3.configure_series_labels().background_style(WHITE.mix(0.8)).border_style(BLACK).draw()?;

    root.present()?;
    Ok(())
}

pub fn render_chart(data: &SimulationData, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Rendering Chart to '{}'...", filename);

    let root = BitMapBackend::new(filename, (CANVAS_WIDTH, CANVAS_HEIGHT_LG)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .caption("Phase 8: Photoreceptor (Analog Sensor) & SchwannCell", ("sans-serif", CAPTION_FONT_SIZE as i32).into_font())
        .margin(CHART_MARGIN)
        .x_label_area_size(LABEL_AREA_SIZE)
        .y_label_area_size(LABEL_AREA_SIZE + 10)
        .right_y_label_area_size(LABEL_AREA_SIZE + 10)
        .build_cartesian_2d(0f32..200f32, VOLTAGE_MIN_F32..SOMA_VOLTAGE_MAX)?
        .set_secondary_coord(0f32..200f32, METRIC_MIN_F32..BUFFER_MAX_F32);
        
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

pub fn render_inhibition_chart(data: &[(u32, f32, f32, f32)], filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Rendering Inhibition Loop Chart to '{}'...", filename);
    let root = BitMapBackend::new(filename, (CANVAS_WIDTH, CANVAS_HEIGHT_LG)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .caption("Inhibition Loop", ("sans-serif", CAPTION_FONT_SIZE as i32).into_font())
        .margin(CHART_MARGIN)
        .x_label_area_size(LABEL_AREA_SIZE)
        .y_label_area_size(LABEL_AREA_SIZE + 10)
        .build_cartesian_2d(0f32..INHIBITION_TIME_LIMIT, VOLTAGE_MIN_F32..VOLTAGE_MAX_F32)?;
    chart.configure_mesh().y_desc("Voltage (mV)").draw()?;
    chart.draw_series(LineSeries::new(data.iter().map(|&(t, v, _, _)| (t as f32, v)), &RED))?
        .label("Pyramidal Voltage").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));
    chart.draw_series(LineSeries::new(data.iter().map(|&(t, _, v, _)| (t as f32, v)), &BLUE))?
        .label("Interneuron Voltage").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));
    chart.configure_series_labels().background_style(WHITE.mix(0.8)).border_style(BLACK).draw()?;
    root.present()?;
    Ok(())
}

pub fn render_reflex_arc_chart(data: &[(u32, f32, f32, f32, f32, f32)], filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Rendering Reflex Arc Chart to '{}'...", filename);
    let root = BitMapBackend::new(filename, (CANVAS_WIDTH, CANVAS_HEIGHT_XL)).into_drawing_area();
    root.fill(&WHITE)?;

    let areas = root.split_evenly((3, 1));

    // Panel 1: Sensory (Eye)
    let mut chart1 = ChartBuilder::on(&areas[0])
        .caption("1. Input: Photoreceptor (The Eye)", ("sans-serif", CAPTION_FONT_SIZE as i32).into_font())
        .margin(CHART_MARGIN).x_label_area_size(LABEL_AREA_SIZE - 10).y_label_area_size(LABEL_AREA_SIZE).build_cartesian_2d(0f32..REFLEX_TIME_LIMIT, EYE_VOLTAGE_LOW..EYE_VOLTAGE_HIGH)?;
    chart1.configure_mesh().y_desc("Voltage (mV)").draw()?;
    chart1.draw_series(LineSeries::new(data.iter().map(|&(t, v, _, _, _, _)| (t as f32, v)), &BLUE))?;

    // Panel 2: Logic (Interneuron / Brain)
    let mut chart2 = ChartBuilder::on(&areas[1])
        .caption("2. Logic: Interneuron (The Brain)", ("sans-serif", CAPTION_FONT_SIZE as i32).into_font())
        .margin(CHART_MARGIN).x_label_area_size(LABEL_AREA_SIZE - 10).y_label_area_size(LABEL_AREA_SIZE).build_cartesian_2d(0f32..REFLEX_TIME_LIMIT, VOLTAGE_MIN_F32..VOLTAGE_MAX_F32)?;
    chart2.configure_mesh().y_desc("Voltage (mV)").draw()?;
    chart2.draw_series(LineSeries::new(data.iter().map(|&(t, _, v, _, _, _)| (t as f32, v)), &MAGENTA))?;

    // Panel 3: Output (Pyramidal / Muscle)
    let mut chart3 = ChartBuilder::on(&areas[2])
        .caption("3. Output: Pyramidal (The Muscle)", ("sans-serif", CAPTION_FONT_SIZE as i32).into_font())
        .margin(CHART_MARGIN).x_label_area_size(LABEL_AREA_SIZE - 10).y_label_area_size(LABEL_AREA_SIZE).right_y_label_area_size(LABEL_AREA_SIZE)
        .build_cartesian_2d(0f32..REFLEX_TIME_LIMIT, VOLTAGE_MIN_F32..VOLTAGE_MAX_F32)?
        .set_secondary_coord(0f32..REFLEX_TIME_LIMIT, METRIC_MIN_F32..(METRIC_MAX_F32 + 10.0));
    
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

pub fn render_plasticity_chart(history: &[(u32, f32, f32)], filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Rendering Plasticity Analysis to '{}'...", filename);
    let root = BitMapBackend::new(filename, (1024, 768)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .caption("Synaptic Plasticity: Dopamine Modulation", ("sans-serif", CAPTION_FONT_SIZE as i32).into_font())
        .margin(CHART_MARGIN + 10).x_label_area_size(LABEL_AREA_SIZE).y_label_area_size(LABEL_AREA_SIZE + 10).right_y_label_area_size(LABEL_AREA_SIZE + 10)
        .build_cartesian_2d(0f32..PLASTICITY_TIME_LIMIT, METRIC_MIN_F32..WEIGHT_MAX_F32)?
        .set_secondary_coord(0f32..PLASTICITY_TIME_LIMIT, METRIC_MIN_F32..DOPAMINE_MAX_F32);

    chart.configure_mesh().y_desc("Synaptic Weight").draw()?;
    chart.configure_secondary_axes().y_desc("Dopamine Level").draw()?;
    
    chart.draw_series(LineSeries::new(history.iter().map(|&(t, w, _)| (t as f32, w)), &RED.mix(0.8)))?
        .label("Synaptic Weight").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));

    chart.draw_secondary_series(AreaSeries::new(history.iter().map(|&(t, _, d)| (t as f32, d)), 0.0, &CYAN.mix(0.2)))?
        .label("Dopamine Multiplier").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], CYAN));

    chart.configure_series_labels().background_style(WHITE.mix(0.8)).border_style(BLACK).draw()?;
    root.present()?;
    Ok(())
}

pub fn render_emergence_chart(data: &[(u32, u32, f32)], filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Rendering Emergence Analysis to '{}'...", filename);
    let root = BitMapBackend::new(filename, (CANVAS_WIDTH, CANVAS_HEIGHT_LG)).into_drawing_area();
    root.fill(&WHITE)?;

    let areas = root.split_evenly((2, 1));

    // Panel 1: Structural Growth
    let max_syn = data.iter().map(|&(_, s, _)| s).max().unwrap_or(100).max(100);
    let mut chart1 = ChartBuilder::on(&areas[0])
        .caption("1. Structural Growth (Synaptogenesis)", ("sans-serif", CAPTION_FONT_SIZE as i32).into_font())
        .margin(CHART_MARGIN).x_label_area_size(LABEL_AREA_SIZE - 10).y_label_area_size(LABEL_AREA_SIZE).build_cartesian_2d(0f32..EMERGENCE_TIME_LIMIT, 0u32..max_syn + 50)?;
    chart1.configure_mesh().y_desc("Synapse Count").draw()?;
    chart1.draw_series(LineSeries::new(data.iter().map(|&(t, s, _)| (t as f32, s)), &GREEN))?
        .label("Active Synapses").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], GREEN));
    chart1.configure_series_labels().background_style(WHITE.mix(0.8)).border_style(BLACK).draw()?;

    // Panel 2: Signal Penetration (Functional Connectivity)
    let mut chart2 = ChartBuilder::on(&areas[1])
        .caption("2. Signal Reach (Remote Neuron Activity)", ("sans-serif", CAPTION_FONT_SIZE as i32).into_font())
        .margin(CHART_MARGIN).x_label_area_size(LABEL_AREA_SIZE - 10).y_label_area_size(LABEL_AREA_SIZE).build_cartesian_2d(0f32..EMERGENCE_TIME_LIMIT, VOLTAGE_MIN_F32..VOLTAGE_MAX_F32)?;
    chart2.configure_mesh().y_desc("Voltage (mV)").draw()?;
    chart2.draw_series(LineSeries::new(data.iter().map(|&(t, _, v)| (t as f32, v)), &BLUE))?
        .label("Neuron 250 Voltage").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));
    chart2.configure_series_labels().background_style(WHITE.mix(0.8)).border_style(BLACK).draw()?;

    root.present()?;
    Ok(())
}

pub fn render_frequency_chart(data: &[(u32, f32, f32, f32, f32)], filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Rendering Frequency Learning Chart to '{}'...", filename);
    let root = BitMapBackend::new(filename, (CANVAS_WIDTH, CANVAS_HEIGHT_LG)).into_drawing_area();
    root.fill(&WHITE)?;
    let areas = root.split_evenly((2, 1));

    // Panel 1: Voltages
    let mut chart1 = ChartBuilder::on(&areas[0])
        .caption("Neuron Responses to Stimuli", ("sans-serif", CAPTION_FONT_SIZE as i32).into_font())
        .margin(CHART_MARGIN).x_label_area_size(LABEL_AREA_SIZE - 10).y_label_area_size(LABEL_AREA_SIZE).build_cartesian_2d(0f32..EMERGENCE_TIME_LIMIT, VOLTAGE_MIN_F32..VOLTAGE_MAX_F32)?;
    chart1.configure_mesh().draw()?;
    chart1.draw_series(LineSeries::new(data.iter().map(|&(t, v1, _, _, _)| (t as f32, v1)), &RED))?
        .label("F1 Detector").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));
    chart1.draw_series(LineSeries::new(data.iter().map(|&(t, _, v2, _, _)| (t as f32, v2)), &BLUE))?
        .label("F2 Detector").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));
    chart1.configure_series_labels().background_style(WHITE.mix(0.8)).draw()?;

    // Panel 2: Synaptic Weights (Plasticity)
    let mut chart2 = ChartBuilder::on(&areas[1])
        .caption("Synaptic Weight Evolution (STDP)", ("sans-serif", CAPTION_FONT_SIZE as i32).into_font())
        .margin(CHART_MARGIN).x_label_area_size(LABEL_AREA_SIZE - 10).y_label_area_size(LABEL_AREA_SIZE).build_cartesian_2d(0f32..EMERGENCE_TIME_LIMIT, METRIC_MIN_F32..35f32)?;
    chart2.configure_mesh().draw()?;
    chart2.draw_series(LineSeries::new(data.iter().map(|&(t, _, _, w1, _)| (t as f32, w1)), &RED))?
        .label("W1 (F1 Weight)").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));
    chart2.draw_series(LineSeries::new(data.iter().map(|&(t, _, _, _, w2)| (t as f32, w2)), &BLUE))?
        .label("W2 (F2 Weight)").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));
    chart2.configure_series_labels().background_style(WHITE.mix(0.8)).draw()?;

    root.present()?;
    Ok(())
}

pub fn render_morphology_chart(data: &[(u32, f32, f32, f32)], filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Rendering Morphology Audit to '{}'...", filename);
    let root = BitMapBackend::new(filename, (CANVAS_WIDTH, CANVAS_HEIGHT_LG)).into_drawing_area();
    root.fill(&WHITE)?;

    let areas = root.split_evenly((2, 1));

    // Panel 1: Synaptic Growth (Weight & Thickness)
    let mut chart1 = ChartBuilder::on(&areas[0])
        .caption("1. Synaptic Growth (Structural Plasticity)", ("sans-serif", CAPTION_FONT_SIZE as i32).into_font())
        .margin(CHART_MARGIN).x_label_area_size(LABEL_AREA_SIZE - 10).y_label_area_size(LABEL_AREA_SIZE).build_cartesian_2d(0f32..MORPHOLOGY_TIME_LIMIT, METRIC_MIN_F32..MAX_SYNAPSE_WEIGHT_BENCHMARK)?;
    chart1.configure_mesh().y_desc("Value").draw()?;
    
    chart1.draw_series(LineSeries::new(data.iter().map(|&(t, w, _, _)| (t as f32, w)), &RED))?
        .label("Syaptic Weight").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));
    
    chart1.configure_series_labels().background_style(WHITE.mix(0.8)).border_style(BLACK).draw()?;

    // Panel 2: Transmission Reliability & Buffer
    let mut chart2 = ChartBuilder::on(&areas[1])
        .caption("2. Transmission Performance & Capacity", ("sans-serif", CAPTION_FONT_SIZE as i32).into_font())
        .margin(CHART_MARGIN).x_label_area_size(LABEL_AREA_SIZE - 10).y_label_area_size(LABEL_AREA_SIZE).build_cartesian_2d(0f32..MORPHOLOGY_TIME_LIMIT, VOLTAGE_MIN_F32..METRIC_MAX_F32)?;
    chart2.configure_mesh().draw()?;
    
    chart2.draw_series(LineSeries::new(data.iter().map(|&(t, _, v, _)| (t as f32, v)), &MAGENTA))?
        .label("Target Voltage").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], MAGENTA));

    chart2.draw_series(LineSeries::new(data.iter().map(|&(t, _, _, buff)| (t as f32, buff)), &GREEN))?
        .label("NT Buffer Volume").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], GREEN));

    chart2.configure_series_labels().background_style(WHITE.mix(0.8)).border_style(BLACK).draw()?;

    root.present()?;
    Ok(())
}
