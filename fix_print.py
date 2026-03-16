import re

with open('cuda/base_neuron.cu', 'r') as f:
    code = f.read()

# Remove the printf blocks
code = re.sub(
    r'#ifndef USE_CUDA\n        if \(n->id == 2\) printf\("Distal branch triggered\. V before: \%\.2f\\n", n->current_voltage\);\n#endif\n',
    r'',
    code
)

code = re.sub(
    r'        #ifndef USE_CUDA\n        if \(n->id == 2 && passive_leak > 0\.1f\) printf\("Branch \%d added passive_leak: \%\.2f\\n", i, passive_leak\);\n        #endif\n',
    r'',
    code
)

code = re.sub(
    r'        #ifndef USE_CUDA\n        if \(n->id == 2 && soma_impact > 0\.1f\) printf\("Branch \%d added soma_impact: \%\.2f\\n", i, soma_impact\);\n        #endif\n',
    r'',
    code
)

code = re.sub(
    r'#ifndef USE_CUDA\n    if \(n->id == 2\) \{\n        printf\("Tick \%d \| id 2 \| V: \%\.2f \| DynThresh: \%\.2f \| ATP: \%\.2f\\n", current_tick, n->current_voltage, dynamic_threshold, n->energy_ATP\);\n    \}\n#endif\n',
    r'',
    code
)

with open('cuda/base_neuron.cu', 'w') as f:
    f.write(code)

