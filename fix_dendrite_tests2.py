import re

with open("tests/dendrite_tests.rs", "r") as f:
    content = f.read()

# Fix the import path for pack_voltage
content = content.replace("crate::engine::ffi::pack_voltage", "neuromorphic_engine::engine::ffi::pack_voltage")

with open("tests/dendrite_tests.rs", "w") as f:
    f.write(content)
