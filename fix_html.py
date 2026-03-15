import re

with open('visualizer/index.html', 'r') as f:
    code = f.read()

code = re.sub(
    r'                        if \(!n\.isAlive\) \{\n                            if \(n\.deathReason === \'calcium\'\) _tempColor\.copy\(TYPE_COLORS\.dead_calcium\);\n                            else if \(n\.deathReason === \'atp\'\) _tempColor\.copy\(TYPE_COLORS\.dead_atp\);\n                            else if \(n\.deathReason === \'regen\'\) _tempColor\.copy\(TYPE_COLORS\.dead_regen\);\n                            else _tempColor\.copy\(TYPE_COLORS\.dead\);\n                        \} else \{\n                            if \(n\.statusFlags & FLAG_IS_INHIBITED\) \{\n                                _tempColor\.copy\(baseColor\)\.multiplyScalar\(0\.1\);\n                            \} else \{\n                                _tempColor\.copy\(baseColor\)\.lerp\(new THREE\.Color\(1,1,1\), n\.visualActivity\);\n                            \}\n                        \}',
    r'''                        if (!n.isAlive) {
                            if (n.deathReason === 'calcium') _tempColor.copy(TYPE_COLORS.dead_calcium);
                            else if (n.deathReason === 'atp') _tempColor.copy(TYPE_COLORS.dead_atp);
                            else if (n.deathReason === 'regen') _tempColor.copy(TYPE_COLORS.dead_regen);
                            else _tempColor.copy(TYPE_COLORS.dead);
                        } else {
                            if (n.statusFlags & FLAG_IS_INHIBITED) {
                                _tempColor.copy(baseColor).multiplyScalar(0.1);
                            } else if (n.statusFlags & FLAG_IS_PREDICTIVE) {
                                // Predictive State: Magenta/Neon Pink (0xff00ff)
                                const predColor = new THREE.Color(0xff00ff);
                                _tempColor.copy(predColor).lerp(new THREE.Color(1,1,1), n.visualActivity);
                            } else {
                                _tempColor.copy(baseColor).lerp(new THREE.Color(1,1,1), n.visualActivity);
                            }
                        }''',
    code
)

with open('visualizer/index.html', 'w') as f:
    f.write(code)

