import re

with open("src/engine/constants.rs", "r") as f:
    content = f.read()

# Add constants
new_constants = """// Dendritic Branches and Columnar Structures
pub const NUM_BRANCHES: usize = 16;
pub const BRANCH_MASK_PROXIMAL: u32 = 0x000F; // Branches 0-3
pub const BRANCH_MASK_DISTAL: u32 = 0x0FF0;   // Branches 4-11
pub const BRANCH_MASK_APICAL: u32 = 0xF000;   // Branches 12-15

// WTA Flags (status_flags)
pub const FLAG_IS_INHIBITED: u32 = 1 << 0;
pub const FLAG_IS_PREDICTIVE: u32 = 1 << 1;
"""

if "NUM_BRANCHES" not in content:
    content += "\n" + new_constants

with open("src/engine/constants.rs", "w") as f:
    f.write(content)

