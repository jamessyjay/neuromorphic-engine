// src/ffi.rs

use libc::c_int;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
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
#[derive(Debug, Clone, Copy)]
pub struct Compartment {
    pub branch_type: u8, // 0=Empty, 1=Basal, 2=Trunk, 3=Tuft
    pub voltage: f32,
    pub nmda_threshold: f32,
    pub _padding: [u8; 3],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BaseNeuron {
    pub id: u32,
    pub type_: NeuronType,
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
    pub spikes_in_last_window: u32,
    pub metabolic_wear: f32,
    pub last_spike_tick: u32,

    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub death_reason: u8,
    pub is_alive: bool,
    pub branches: [Compartment; 8],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
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
#[derive(Debug, Clone, Copy)]
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
