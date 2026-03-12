// src/stimulus.rs
use crate::graph::NetworkGraph;
use crate::ffi::Environment;

pub struct StimulusService {
    pub stimulus_interval: u32,
    pub response_window: u32,
    pub last_stimulus_tick: u32,
    pub waiting_for_response: bool,
}

impl StimulusService {
    pub fn new(interval: u32, window: u32) -> Self {
        Self {
            stimulus_interval: interval,
            response_window: window,
            last_stimulus_tick: 0,
            waiting_for_response: false,
        }
    }

    pub fn process_tick(&mut self, graph: &mut NetworkGraph, env: &mut Environment, current_tick: u32) {
        let num_neurons = graph.neurons.len();
        if num_neurons == 0 { return; }
        let num_sensors = (num_neurons as f32 * 0.02) as usize;
        let num_motors = (num_neurons as f32 * 0.02) as usize;
        let motor_start_idx = num_neurons.saturating_sub(num_motors);

        // Phase A: The Flash (Stimulus)
        if current_tick % self.stimulus_interval == 0 {
            // Apply stimulus to the first 2% (Sensors)
            for i in 0..num_sensors {
                if i < graph.neurons.len() {
                    let n = &mut graph.neurons[i];
                    if n.is_alive {
                        n.current_voltage = 40.0; // Spike voltage
                        n.energy_atp = n.max_atp; // Full energy for reliable spike
                    }
                }
            }
            self.last_stimulus_tick = current_tick;
            self.waiting_for_response = true;
            
            // Set a "learning floor" - dopamine drops to force pruning of non-contributors
            env.dopamine_multiplier = 0.5; 
        }

        // Phase B: The Observation (Response Window)
        if self.waiting_for_response {
            let mut response_detected = false;
            
            // Check if any motor neuron spiked
            for i in motor_start_idx..num_neurons {
                if graph.neurons[i].last_spike_tick == current_tick {
                    response_detected = true;
                    break;
                }
            }

            if response_detected {
                // Success!
                env.dopamine_multiplier = 8.0; // EVEN BIGGER RUSH for visibility
                self.waiting_for_response = false;
                println!("[DOPAMINE RUSH] Motor Response Detected at tick {}! Rewarding network.", current_tick);
            } else if current_tick > self.last_stimulus_tick + self.response_window {
                // Window expired
                self.waiting_for_response = false;
                env.dopamine_multiplier = 0.5; 
            }
        }

        // Phase C: Dopamine Decay (Smooth transition)
        // If dopamine is high, let it decay back to baseline (1.0)
        if env.dopamine_multiplier > 1.0 {
            env.dopamine_multiplier -= 0.05; // 5% decay per tick
        } else if env.dopamine_multiplier < 1.0 && !self.waiting_for_response {
            env.dopamine_multiplier += 0.01; // Recover to baseline
        }
    }
}
