// src/telemetry.rs

use std::time::Instant;
use std::fs;
use crate::engine::ffi::{BaseNeuron, Synapse};
use crate::engine::constants::*;

pub struct Telemetry {
    start_time: Instant,
    total_ticks: u32,
    
    // Performance metrics
    pub tps: f64,
    pub sim_speed: f64, // Simulated time / Real time (1.0 = realtime)
}

impl Telemetry {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            total_ticks: 0,
            tps: 0.0,
            sim_speed: 0.0,
        }
    }

    pub fn update(&mut self, current_tick: u32) {
        self.total_ticks = current_tick;
        let elapsed = self.start_time.elapsed();
        if elapsed.as_secs_f64() > 0.0 {
            self.tps = current_tick as f64 / elapsed.as_secs_f64();
            // Assuming 1 tick = 1ms of biological time
            self.sim_speed = (current_tick as f64 * SECONDS_PER_TICK) / elapsed.as_secs_f64();
        }
    }

    pub fn print_report(&self, num_neurons: usize, num_synapses: usize, dead_count: usize) {
        println!("\n--- TELEMETRY REPORT ---");
        println!("Status: {} Alive | {} Dead (Total: {})", num_neurons - dead_count, dead_count, num_neurons);
        println!("Simulation Speed: {:.2}x ({})", 
            self.sim_speed, 
            if self.sim_speed >= 1.0 { "FASTER THAN REALTIME" } else { "SLOWER THAN REALTIME" }
        );
        println!("Processing Speed: {:.2} TPS", self.tps);
        
        let neuron_mem = std::mem::size_of::<BaseNeuron>();
        let synapse_mem = std::mem::size_of::<Synapse>();
        let total_mem_kb = (num_neurons * neuron_mem + num_synapses * synapse_mem) as f32 / 1024.0;
        
        println!("Memory Overhead: {:.2} KB total", total_mem_kb);
        println!("  - Per Neuron: {} bytes", neuron_mem);
        println!("  - Per Synapse: {} bytes", synapse_mem);
        
        // Jetson/Tegrastats Mock
        if let Ok(content) = fs::read_to_string("/sys/devices/virtual/thermal/thermal_zone0/temp") {
            println!("SoC Temp: {} C", content.trim().parse::<f32>().unwrap_or(0.0) / 1000.0);
        } else {
            println!("Hardware Telemetry: (Mock Mode / Not on Jetson)");
        }
        println!("------------------------\n");
    }

    pub fn export_simulation_report(&self, neurons: &[BaseNeuron], synapses: &[Synapse], filename: &str) {
        let _ = fs::create_dir_all("reports");
        println!("Exporting population report to '{}'...", filename);
        
        // 1. Death Reasons
        let mut atp_deaths = 0;
        let mut calcium_deaths = 0;
        let mut glucose_deaths = 0;
        
        for n in neurons {
            match n.death_reason {
                1 => atp_deaths += 1,
                2 => calcium_deaths += 1,
                3 => glucose_deaths += 1,
                _ => {}
            }
        }
        
        // 2. Distributions
        let mut weight_bins = vec![0; WEIGHT_BINS_COUNT];
        let thickness_bins = vec![0; WEIGHT_BINS_COUNT];
        let mut atp_bins = vec![0; ATP_BINS_COUNT]; // e.g. 0-20, 20-40, 40-60, 60-80, 80-100
        
        let weight_bin_size = (MAX_SYNAPSE_WEIGHT / WEIGHT_BINS_COUNT as f32) as usize;
        for s in synapses {
            let w_idx = (s.weight as usize / weight_bin_size).min(WEIGHT_BINS_COUNT - 1);
            weight_bins[w_idx] += 1;
            
            // Removed thickness tracking
        }
        
        let mut low_atp_count = 0;
        let atp_bin_size = (MAX_ATP / ATP_BINS_COUNT as f32) as usize;
        for n in neurons {
            if n.is_alive == 1 {
                let _wear_f = n.metabolic_wear as f32 / MAX_U16_AS_F32;
                let a_idx = (n.energy_atp as usize / atp_bin_size).min(ATP_BINS_COUNT - 1);
                atp_bins[a_idx] += 1;
                if n.energy_atp < EMERGENCY_ATP_THRESHOLD { low_atp_count += 1; }
            }
        }
        
        // 3. JSON Generation (Manual for zero-dependency)
        let json = format!(r#"{{
  "simulation_summary": {{
    "total_ticks": {},
    "sim_speed": {:.2},
    "final_neuron_count": {},
    "final_synapse_count": {}
  }},
  "incident_log": {{
    "atp_low_deaths": {},
    "calcium_high_deaths": {},
    "glucose_starvation_deaths": {},
    "neurons_in_emergency_atp": {}
  }},
  "population_distributions": {{
    "synaptic_weights": {:?},
    "synaptic_thickness": {:?},
    "neuron_atp_bins": {:?}
  }}
}}"#, 
            self.total_ticks, self.sim_speed, neurons.len(), synapses.len(),
            atp_deaths, calcium_deaths, glucose_deaths, low_atp_count,
            weight_bins, thickness_bins, atp_bins
        );
        
        let _ = fs::write(filename, json);
        println!("Report generated successfully.");
    }
}
