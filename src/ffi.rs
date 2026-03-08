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
    pub is_alive: bool,
    
    // Phase 2 & 13: Biological States
    pub calcium_level: f32,
    pub vesicles_pool: f32,
    pub refractory_time_left: f32,
    
    // Evolution & Adaptation (Phase 13)
    pub spikes_in_last_window: u32,
    pub metabolic_wear: f32,

    pub last_spike_tick: u32,

    pub inputs_count: u32,
    pub outputs_count: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Synapse {
    pub source_id: u32,
    pub target_id: u32,
    pub weight: f32,
    pub neurotransmitter_buffer: f32,
    pub ticks_until_delivery: i32,
    pub is_inhibitory: bool,
    pub last_activation_tick: u32,
    pub is_myelinated: bool,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Environment {
    pub global_oxygen_level: f32,
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
