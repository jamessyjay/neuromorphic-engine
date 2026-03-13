import re

with open("tests/test_stdp.rs", "r") as f:
    content = f.read()

new_debug = """        // Rest for a bit to avoid continuous spiking and allow traces to settle
        for _ in 0..80 {
            graph.tick(tick, env.clone());
            for b in 10..=15 {
                graph.neurons[b as usize].spikes_in_last_window = 10;
            }
            tick += 1;
        }
        if _cycle % 100 == 0 {
            let avg = a_to_b.iter().map(|&i| graph.synapses[i].weight).sum::<f32>() / a_to_b.len() as f32;
            println!("Cycle {} A->B weight: {:.4}", _cycle, avg);
        }"""

content = content.replace("""        // Rest for a bit to avoid continuous spiking and allow traces to settle
        for _ in 0..80 {
            graph.tick(tick, env.clone());
            for b in 10..=15 {
                graph.neurons[b as usize].spikes_in_last_window = 10;
            }
            tick += 1;
        }""", new_debug)

with open("tests/test_stdp.rs", "w") as f:
    f.write(content)
