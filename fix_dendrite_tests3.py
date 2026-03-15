import re

with open("tests/dendrite_tests.rs", "r") as f:
    content = f.read()

# Since `pack_voltage` is not exported from ffi or it's a C++ only function... wait!
# Let's check where `pack_voltage` is.
# In `tests/dendrite_tests.rs`, we already use a function called `pack_voltage` in `test_nmda_spike_trigger`.
# Let's see how it's defined there.
