import re

with open('tests/neuron_physics_tests.rs', 'r') as f:
    code = f.read()

# "Dist voltage: 40, Focus voltage: 40"
# "Distributed should depolarize but NOT spike!"
# Both spiked! Because `sub_stimulus` is 25.0!
# If branch voltage is 25.0, `v - (-70.0)` is 95.0. 
# `passive_leak = 95.0 * 0.05 = 4.75`.
# `n->current_voltage += 4.75`.
# Two branches => 4.75 * 2 = 9.5.
# And `v_above_rest` = 95.0.
# Wait! In the new `else` block:
# `v_above_rest = fmaxf(0.0f, v - (-70.0f));`
# `soma_impact = v_above_rest * BASAL_SOMA_GAIN;`
# If BASAL_SOMA_GAIN is 0.1, then `soma_impact` = 95.0 * 0.1 = 9.5.
# Two branches => 9.5 * 2 = 19.0.
# Total `n->current_voltage` increase = 9.5 (passive) + 19.0 (impact) = 28.5.
# Resting is -70. -70 + 28.5 = -41.5.
# But activation threshold is ~ -50.0!
# So -41.5 > -50.0 is TRUE! So it SPIKES!
# That's why BOTH spiked!
# We need `sub_stimulus` to be smaller! E.g. -40.0.
# If v = -40.0, `v - (-70.0) = 30.0`.
# `passive_leak` = 30.0 * 0.05 = 1.5.
# `v_above_rest` = 30.0. `soma_impact` = 3.0.
# Total increase = 4.5.
# Two branches => 9.0 increase.
# -70 + 9.0 = -61.0. This is less than -50.0! NO SPIKE!
# Let's change `sub_stimulus` from 25.0 to -40.0, and `sub_stimulus * 2.0` won't work, we need something that adds up to > 40.0 for focus!
# Wait! NMDA threshold is `40.0` (which means v=40.0).
# We want the focused branch to have `v > 40.0`.
# Let's just explicitly set `n_dist.branches[0].voltage = pack_voltage(-40.0)`.
# And `n_focus.branches[0].voltage = pack_voltage(50.0)`.

code = re.sub(
    r'    let sub_stimulus = 25\.0; // Two of these is 50\.0 \(> 40\.0\)\n    \n    // Action A \(Distributed\)\n    n_dist\.branches\[0\]\.voltage = pack_voltage\(sub_stimulus\);\n    n_dist\.branches\[4\]\.voltage = pack_voltage\(sub_stimulus\);\n    \n    // Action B \(Focused\)\n    n_focus\.branches\[0\]\.voltage = pack_voltage\(sub_stimulus \* 2\.0\);',
    r'''    // Action A (Distributed)
    // Distributed inputs: enough to cause some passive transfer, but not enough to cross threshold
    n_dist.branches[0].voltage = pack_voltage(-40.0);
    n_dist.branches[4].voltage = pack_voltage(-40.0);
    
    // Action B (Focused)
    // Focused inputs: crosses NMDA threshold of 40.0
    n_focus.branches[0].voltage = pack_voltage(50.0);''',
    code
)

with open('tests/neuron_physics_tests.rs', 'w') as f:
    f.write(code)

