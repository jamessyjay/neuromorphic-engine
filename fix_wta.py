import re

with open('tests/columnar_tests.rs', 'r') as f:
    code = f.read()

code = re.sub(
    r'    // Run tick \(WTA lateral inhibition applies BEFORE physics\)\n    graph\.apply_wta_inhibition\(\);\n    println!\("Before tick, Loser: v=\{\}", graph\.neurons\[2\]\.current_voltage\);\n    graph\.tick\(1, env\);',
    r'    // Run tick (WTA lateral inhibition applies during physics phase 2)\n    graph.tick(1, env);',
    code
)

with open('tests/columnar_tests.rs', 'w') as f:
    f.write(code)

