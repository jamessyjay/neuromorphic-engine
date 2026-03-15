import re

with open('visualizer/index.html', 'r') as f:
    code = f.read()

code = re.sub(
    r'                for \(let i = 0; i < MAX_PULSES; i\+\+\) \{\n                    const idx = i\*3;\n                    if \(i < activePulses\.length\) \{',
    r'''                for (let i = 0; i < MAX_PULSES; i++) {
                    const idx = i*3;
                    if (i < activePulses.length) {
                        // Check for bursting (represented by column-wide flash)
                        // If we wanted pure vertical flash, we could spawn special pulses, but for now we just 
                        // rely on the massive spike wave triggering many activePulses automatically.''',
    code
)

with open('visualizer/index.html', 'w') as f:
    f.write(code)

