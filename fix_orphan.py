import re

with open('src/runner/modes.rs', 'r') as f:
    code = f.read()

code = re.sub(
    r'                let target_id = candidates\[k\]\.0 as u32;\n                // Add to outgoing_synapses to prevent next checks from reusing it if logic changes\n                outgoing_synapses\[i\]\.push\(target_id\); \n                \n                let is_inhib = graph\.neurons\[i\]\.type_ == crate::engine::ffi::NeuronType::Interneuron as u8;\n                let w = if is_inhib \{ -2\.5 \} else \{ 2\.5 \};\n                new_synapses\.push\(\(i as u32, target_id, w, is_inhib\)\);',
    r'''                let target_id = candidates[k].0 as u32;
                
                // Extra safety: Enforce L6->L5 ban even during orphan saving
                if graph.neurons[i].layer_id == 6 && graph.neurons[candidates[k].0].layer_id == 5 {
                    continue;
                }

                // Add to outgoing_synapses to prevent next checks from reusing it if logic changes
                outgoing_synapses[i].push(target_id); 
                
                let is_inhib = graph.neurons[i].type_ == crate::engine::ffi::NeuronType::Interneuron as u8;
                let w = if is_inhib { -2.5 } else { 2.5 };
                new_synapses.push((i as u32, target_id, w, is_inhib));''',
    code
)

with open('src/runner/modes.rs', 'w') as f:
    f.write(code)

