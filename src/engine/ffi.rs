// src/ffi.rs

use libc::c_int;
use serde::{Serialize, Deserialize};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NeuronType {
    Pyramidal = 0,
    Bipolar = 1,
    Interneuron = 2,
    Pacemaker = 3,
    Purkinje = 4,
    Photoreceptor = 5,
    Nociceptor = 6,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Compartment {
    pub branch_type: u8,
    pub refractory_timer: u8,
    pub voltage: u16,         // Fixed-point
    pub nmda_threshold: u16,  // Fixed-point
    pub local_trace: u8,
    pub _pad: u8,
}

#[repr(C, align(64))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BaseNeuron {
    // --- Block 1 (0-63 bytes): Identification and Physics ---
    pub id: u32,
    pub type_: u8,
    pub death_reason: u8,
    pub is_alive: u8,
    pub internal_phase: u8,
    
    pub column_id: u16,
    pub minicolumn_id: u8,
    pub layer_id: u8,
    pub status_flags: u32,
    
    pub current_voltage: f32,
    pub activation_threshold: f32,
    pub resting_voltage: f32,
    pub spike_trace: f32,
    pub external_input: f32,
    
    pub _pad_block1: [u8; 28],
    
    // --- Block 2 (64-127 bytes): Biochemistry and Reference Frames ---
    pub energy_atp: f32,
    pub max_atp: f32,
    pub leak_coeff: f32,
    pub calcium_level: f32,
    
    pub vesicles_pool: f32,
    pub refractory_time_left: f32,
    pub ref_coord_x: f32,
    pub ref_coord_y: f32,
    
    pub x: u16,
    pub y: u16,
    pub z: u16,
    pub dead_ticks: u16,
    
    pub metabolic_wear: u16,
    pub spikes_in_last_window: u8,
    pub predictive_ticks: u8,
    pub last_spike_tick: u32, // Changed order to fit 4 bytes correctly without padding issues if any, wait, 2+1+1=4, 4+4=8. So metabolic_wear(2)+spikes(1)+pred(1) = 4 bytes. last_spike(4) = 4 bytes. Perfect 8 byte alignment.
    
    pub _pad_block2: [u8; 16],
    
    // --- Blocks 3-4 (128-255 bytes): Expanded Dendritic Array ---
    pub branches: [Compartment; 16],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Synapse {
    pub source_id: u32,
    pub target_id: u32,
    pub weight: f32,
    pub neurotransmitter_buffer: f32,
    pub last_activation_tick: u32,
    pub ticks_below_threshold: u32,
    pub ticks_until_delivery: u8,
    pub flags: u8, // Bits 0-2: branch_id, Bit 3: is_inhibitory, Bit 4: is_myelinated
    pub _padding: [u8; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Environment {
    pub global_oxygen_level: f32,
    pub global_glucose_level: f32,
    pub dopamine_multiplier: f32,
    pub toxin_levels: f32,
    pub light_level: f32,
    pub stdp_learning_rate: f32,
    pub wakefulness_multiplier: f32,
}

#[link(name = "neuron_kernels", kind = "static")]
extern "C" {
    pub fn launch_tick_neurons(
        neurons: *mut BaseNeuron, 
        count: c_int,
        synapses: *mut Synapse,
        syn_count: c_int,
        env: Environment,
        current_tick: u32
    );
    
    pub fn launch_astrocyte_service(
        neurons: *mut BaseNeuron, 
        count: c_int,
        synapses: *mut Synapse,
        syn_count: c_int,
        env: Environment
    );

    pub fn launch_microglia_service(
        neurons: *mut BaseNeuron, 
        count: c_int,
        synapses: *mut Synapse,
        syn_count: c_int,
        current_tick: u32
    );
    
    // CPU Fallback multi-threading bindings
    pub fn tick_neuron_prepare_cpu(neuron: *mut BaseNeuron, env: Environment, current_tick: u32);
    pub fn tick_neuron_spike_cpu(neuron: *mut BaseNeuron, env: Environment, current_tick: u32);
    pub fn tick_synapse_cpu(synapse: *mut Synapse, neurons: *mut BaseNeuron, env: Environment, current_tick: u32);

    pub fn init_neuron(neuron: *mut BaseNeuron, id: u32, type_: NeuronType, seed: u32);
    pub fn synchronize_device();
    pub fn cleanup_gpu_resources(ptr: *mut std::ffi::c_void);
    pub fn debug_get_minicolumn_winner(column_id: u16, minicolumn_id: u8, out_v: *mut f32, out_id: *mut u32);
}

#[cfg(test)]
mod ffi_tests {
    use super::*;

    #[test]
    fn test_memory_alignment() {
        assert_eq!(std::mem::size_of::<Compartment>(), 8, "Compartment must be exactly 8 bytes");
        assert_eq!(std::mem::size_of::<BaseNeuron>(), 256, "BaseNeuron must be exactly 256 bytes");
    }
}
