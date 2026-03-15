import re

with open("src/network/graph.rs", "r") as f:
    content = f.read()

# OH! Look at graph.rs!
# `pub fn tick()` doesn't call `apply_wta_inhibition`?!
# I replaced `pub fn tick` earlier but wait!
# Let me look at the `git diff` from earlier!
# In `fix_graph_wta.py`, I did:
# `content = re.sub(r'pub fn tick\(&mut self, current_tick: u32, env: &mut Environment\) \{', r'pub fn tick(&mut self, current_tick: u32, env: &mut Environment) {\n        // Apply WTA lateral inhibition before physics (v0.5.0)\n        self.apply_wta_inhibition();\n', content)`
# BUT I changed `env: &mut Environment` to `env: crate::engine::ffi::Environment` across files!!!
# SO `fix_graph_wta.py` regex FAILED to match, and `apply_wta_inhibition()` WAS NEVER ADDED TO `pub fn tick`!
# The only place `apply_wta_inhibition()` was called was inside the tests manually!!
# THAT explains EVERYTHING!

content = content.replace("pub fn tick(&mut self, current_tick: u32, env: crate::engine::ffi::Environment) {", "pub fn tick(&mut self, current_tick: u32, env: crate::engine::ffi::Environment) {\n        self.apply_wta_inhibition();")

with open("src/network/graph.rs", "w") as f:
    f.write(content)

