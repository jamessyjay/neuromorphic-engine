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
    pub fn save_to_disk(&self, filename: &str) {
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
                Ok(snapshot) => Some(snapshot),
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
