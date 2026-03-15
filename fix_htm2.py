import re

with open('tests/htm_tests.rs', 'r') as f:
    code = f.read()

# We need to run more ticks to let the signal deliver, and also ensure the threshold is crossed properly.
# The branch voltage is 150.0 right now, but maybe it needs a few more ticks to be evaluated and set predictive state.

code = re.sub(
    r'    // 11th time: only Pattern 1 fires\n    graph\.neurons\[0\]\.current_voltage = 50\.0;\n    graph\.tick\(200, env\.clone\(\)\);\n    graph\.tick\(201, env\.clone\(\)\); // Delivery\n    graph\.tick\(202, env\.clone\(\)\); // Evaluation',
    r'''    // 11th time: only Pattern 1 fires
    graph.neurons[0].current_voltage = 50.0;
    graph.tick(200, env.clone());
    graph.tick(201, env.clone()); // Delivery
    graph.tick(202, env.clone()); // Summation
    graph.tick(203, env.clone()); // Evaluation & Predictive set
    graph.tick(204, env.clone());''',
    code
)

with open('tests/htm_tests.rs', 'w') as f:
    f.write(code)

