import re

with open('tests/test_stdp.rs', 'r') as f:
    code = f.read()

code = re.sub(
    r'                source_id: src, target_id: tgt, weight: 1\.0,\n                neurotransmitter_buffer: 0\.0, last_activation_tick: 0,',
    r'''                source_id: src, target_id: tgt, weight: 0.1,
                neurotransmitter_buffer: 0.0, last_activation_tick: 0,''',
    code
)

code = re.sub(
    r'                println!\("Quiet Spike at Tick \{\}: Neuron \{\} \(V: \{:\.2\}, Thresh: \{:\.2\}\)", tick, n\.id, n\.current_voltage, n\.activation_threshold\);\n',
    r'',
    code
)

with open('tests/test_stdp.rs', 'w') as f:
    f.write(code)

