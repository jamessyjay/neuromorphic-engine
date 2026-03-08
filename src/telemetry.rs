// src/telemetry.rs

use std::time::Instant;
use std::fs;
use crate::ffi::{BaseNeuron, Synapse};

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
            self.sim_speed = (current_tick as f64 * 0.001) / elapsed.as_secs_f64();
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
}
