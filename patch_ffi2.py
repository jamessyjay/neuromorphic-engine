import re

with open("src/engine/ffi.rs", "r") as f:
    content = f.read()

# Make sure we add `assert_eq!` check inside the file so we can catch it with cargo test
content = content + """
#[cfg(test)]
mod ffi_tests {
    use super::*;

    #[test]
    fn test_memory_alignment() {
        assert_eq!(std::mem::size_of::<Compartment>(), 8, "Compartment must be exactly 8 bytes");
        assert_eq!(std::mem::size_of::<BaseNeuron>(), 256, "BaseNeuron must be exactly 256 bytes");
    }
}
"""

with open("src/engine/ffi.rs", "w") as f:
    f.write(content)

