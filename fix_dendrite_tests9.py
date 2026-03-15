import re

with open("tests/dendrite_tests.rs", "r") as f:
    content = f.read()

# Proximal is failing.
# `Voltage after Proximal: -79.95146`
# This means the neuron DID SPIKE!
# Because after a spike, it goes into refractory period: `n->current_voltage = n->resting_voltage - HYPERPOLARIZATION_OFFSET;`
# -70.22 - 10.0 = -80.22. With some passive leak/noise, it is -79.95.
# So Proximal DOES cause an action potential!
# But the assert was looking for `current_voltage > 30.0` (ACTION_POTENTIAL_PEAK).
# BUT `graph.tick(6)` does `process_spike`, which sets it to 40.0, and then the NEXT tick it would drop...
# Wait! In `process_spike`: `if (spiked) { n->current_voltage = ACTION_POTENTIAL_PEAK; }`
# If we check `current_voltage` right after `tick(6)`, it SHOULD be 40.0!
# Why is it -79.95?
# Let's see the tick sequence:
# graph.tick(4, env);  // source spikes
# graph.tick(5, env);  // neurotransmitter delivered
# graph.tick(6, env);  // summation -> target spikes! current_voltage = 40.0
# Wait! Did target spike at tick 5 or 6?
# In tick 5: `evaluate_branches` sees delivery. NMDA threshold crossed. `soma_impact` = 20.0 * 5.0 = 100.0.
# `current_voltage` becomes -50.0 + 100.0 = +50.0.
# `process_spike` sees 50.0 > threshold. SPIKE! `current_voltage = 40.0`. `refractory_time_left = 2.0`.
# In tick 6: `process_spike` sees `refractory_time_left > 0`. 
# So it does: `n->current_voltage = n->resting_voltage - HYPERPOLARIZATION_OFFSET;` (-80.0) !
# Ah! It spiked on tick 5!

# So we can just check `last_spike_tick` to confirm it spiked!
content = content.replace('println!("Voltage after Proximal: {}", graph.neurons[2].current_voltage);\n    assert!(graph.neurons[2].current_voltage > 30.0, "Proximal must cause an action potential (spike peak)");', 'println!("Voltage after Proximal: {}, Last Spike Tick: {}", graph.neurons[2].current_voltage, graph.neurons[2].last_spike_tick);\n    assert!(graph.neurons[2].last_spike_tick >= 4, "Proximal must cause an action potential (spike peak)");')

with open("tests/dendrite_tests.rs", "w") as f:
    f.write(content)

