import re

with open('tests/synapse_tests.rs', 'r') as f:
    code = f.read()

# Since we mock the spike logic here by manually setting the voltage and last_spike_tick,
# the new graph.tick() might be overwriting the manual state or doing something different.
# We need to make sure we don't clear the spike trace right after setting it, or that we allow the tick to process the spike naturally.
code = re.sub(
    r'    graph\.neurons\[src as usize\]\.current_voltage = 40\.0;\n    graph\.neurons\[src as usize\]\.last_spike_tick = 1;\n    graph\.tick\(1, env\.clone\(\)\);',
    r'    graph.neurons[src as usize].current_voltage = 50.0;\n    graph.tick(1, env.clone());',
    code
)

code = re.sub(
    r'    graph\.neurons\[tgt as usize\]\.current_voltage = 40\.0;\n    graph\.neurons\[tgt as usize\]\.last_spike_tick = 6;\n    graph\.neurons\[tgt as usize\]\.branches\[0\]\.local_trace = 255;\n    graph\.tick\(6, env\.clone\(\)\);',
    r'    graph.neurons[tgt as usize].current_voltage = 50.0;\n    graph.neurons[tgt as usize].branches[0].local_trace = 255;\n    graph.tick(6, env.clone());',
    code
)

with open('tests/synapse_tests.rs', 'w') as f:
    f.write(code)

