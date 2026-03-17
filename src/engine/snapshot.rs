use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::{BufReader, BufWriter};

use crate::network::graph::NetworkGraph;
use crate::engine::ffi::Environment;

#[derive(Serialize, Deserialize)]
pub struct BrainSnapshot {
    pub current_tick: u32,
    pub environment: Environment,
    pub graph: NetworkGraph,
}

impl BrainSnapshot {
    pub fn save_test_snapshot(tick: u32, graph: &NetworkGraph, save_file: &str) {
        Self {
            current_tick: tick,
            environment: Environment { 
                global_oxygen_level: 1.0, 
                global_glucose_level: 1.0, 
                dopamine_multiplier: 1.0, 
                toxin_levels: 0.0, 
                light_level: 0.0, 
                stdp_learning_rate: 0.01, 
                wakefulness_multiplier: 1.0 
            },
            graph: graph.clone()
        }.save_to_disk(save_file);
    }
    
    pub fn save_to_disk(&self, filename: &str) {
        unsafe { crate::engine::ffi::synchronize_device(); }

        println!("  -> Saving true snapshot to disk ({})...", filename);
        let file = File::create(filename).unwrap_or_else(|_| panic!("Failed to create {}", filename));
        let writer = BufWriter::new(file);
        bincode::serialize_into(writer, &self).expect("Failed to serialize true snapshot");
    }

    pub fn load_from_disk(filename: &str) -> Option<Self> {
        if std::path::Path::new(filename).exists() {
            let file = File::open(filename).unwrap_or_else(|_| panic!("Failed to open {}", filename));
            let reader = BufReader::new(file);
            match bincode::deserialize_from(reader) {
                Ok(snapshot) => {
                    let mut snap: BrainSnapshot = snapshot;
                    snap.graph.optimize_memory_layout();
                    Some(snap)
                },
                Err(e) => {
                    eprintln!("Failed to deserialize {}: {}", filename, e);
                    None
                }
            }
        } else {
            None
        }
    }
}
