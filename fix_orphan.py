import re

with open('src/runner/modes.rs', 'r') as f:
    code = f.read()

code = re.sub(
    r'            for k in 0\.\.needed\.min\(candidates\.len\(\)\) \{\n                let target_id = candidates\[k\]\.0 as u32;\n                \n                // Extra safety: Enforce L6->L5 ban even during orphan saving\n                if graph\.neurons\[i\]\.layer_id == 6 && graph\.neurons\[candidates\[k\]\.0\]\.layer_id == 5 \{\n                    continue;\n                \}',
    r'''            let mut added = 0;
            for k in 0..candidates.len() {
                if added >= needed { break; }
                let target_id = candidates[k].0 as u32;
                
                // Extra safety: Enforce L6->L5 ban even during orphan saving
                if graph.neurons[i].layer_id == 6 && graph.neurons[candidates[k].0].layer_id == 5 {
                    continue;
                }''',
    code
)

code = re.sub(
    r'                let is_inhib = graph\.neurons\[i\]\.type_ == crate::engine::ffi::NeuronType::Interneuron as u8;\n                let w = if is_inhib \{ -2\.5 \} else \{ 2\.5 \};\n                new_synapses\.push\(\(i as u32, target_id, w, is_inhib\)\);\n            \}',
    r'''                let is_inhib = graph.neurons[i].type_ == crate::engine::ffi::NeuronType::Interneuron as u8;
                let w = if is_inhib { -2.5 } else { 2.5 };
                new_synapses.push((i as u32, target_id, w, is_inhib));
                added += 1;
            }''',
    code
)

with open('src/runner/modes.rs', 'w') as f:
    f.write(code)

