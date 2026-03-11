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
    pub _padding: [u8; 2],
}

#[repr(C)]
#[repr(align(64))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BaseNeuron {
    pub id: u32,
    pub type_: u8,            // NeuronType
    pub death_reason: u8,
    pub is_alive: bool,
    pub _pad1: u8,
    
    pub size: f32,
    pub current_voltage: f32,
    
    pub activation_threshold: f32,
    pub resting_voltage: f32,

    pub energy_atp: f32,
    pub max_atp: f32,
    pub leak_coeff: f32,
    
    // Biological States
    pub calcium_level: f32,
    pub vesicles_pool: f32,
    pub refractory_time_left: f32,
    
    // Evolution & Adaptation
    pub last_spike_tick: u32,
    pub spikes_in_last_window: u16,
    pub metabolic_wear: u16,

    pub x: u16,
    pub y: u16,
    pub z: u16,
    pub _pad2: u16,
    
    pub branches: [Compartment; 8],
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
    pub seed: u32,
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
    
    pub fn init_neuron(neuron: *mut BaseNeuron, id: u32, type_: NeuronType, seed: u32);
}
