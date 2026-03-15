import re

with open("cuda/base_neuron.h", "r") as f:
    content = f.read()

new_struct = """struct __align__(64) BaseNeuron {
    // --- Block 1 (0-63 bytes): Identification and Physics ---
    uint32_t id;
    uint8_t type_;
    uint8_t death_reason;
    uint8_t is_alive;
    uint8_t internal_phase;
    
    uint16_t column_id;
    uint8_t minicolumn_id;
    uint8_t layer_id;
    uint32_t status_flags;
    
    float current_voltage;
    float activation_threshold;
    float resting_voltage;
    float spike_trace;
    
    uint8_t _pad_block1[32];
    
    // --- Block 2 (64-127 bytes): Biochemistry and Reference Frames ---
    float energy_ATP;
    float max_atp;
    float leak_coeff;
    float calcium_level;
    
    float vesicles_pool;
    float refractory_time_left;
    float ref_coord_x;
    float ref_coord_y;
    
    uint16_t x;
    uint16_t y;
    uint16_t z;
    uint16_t dead_ticks;
    
    uint16_t metabolic_wear;
    uint8_t spikes_in_last_window;
    uint8_t predictive_ticks;
    uint32_t last_spike_tick;
    
    uint8_t _pad_block2[16];
    
    // --- Blocks 3-4 (128-255 bytes): Expanded Dendritic Array ---
    Compartment branches[16];
};"""

pattern = r'struct __align__\(64\) BaseNeuron \{[\s\S]*?Compartment branches\[8\];\n\};'
content = re.sub(pattern, new_struct, content)

with open("cuda/base_neuron.h", "w") as f:
    f.write(content)
