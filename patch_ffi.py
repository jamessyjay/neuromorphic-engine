import re

with open("src/engine/ffi.rs", "r") as f:
    content = f.read()

# Replace BaseNeuron struct
pattern = r'#\[repr\(C, align\(64\)\)\]\s*#\[derive\(Debug, Clone, Copy, Serialize, Deserialize\)\]\s*pub struct BaseNeuron \{[\s\S]*?pub branches: \[Compartment; 8\],\s*\}'

new_struct = """#[repr(C, align(64))]
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
    
    pub _pad_block1: [u8; 32],
    
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
}"""

content = re.sub(pattern, new_struct, content)

with open("src/engine/ffi.rs", "w") as f:
    f.write(content)
