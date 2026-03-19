use neuromorphic_engine::engine::ffi::Environment;
use std::io::Write;

// --- Test 1: Parser Logic ---
// We write a fake MNIST binary file to temp storage and test the loader.
#[test]
fn test_mnist_parser_logic() {
    let mut temp_path = std::env::temp_dir();
    temp_path.push("fake_mnist.idx3");
    
    let mut f = std::fs::File::create(&temp_path).unwrap();
    
    // Header
    let magic: u32 = 2051;
    let num_images: u32 = 1;
    let rows: u32 = 2;
    let cols: u32 = 2;
    
    f.write_all(&magic.to_be_bytes()).unwrap();
    f.write_all(&num_images.to_be_bytes()).unwrap();
    f.write_all(&rows.to_be_bytes()).unwrap();
    f.write_all(&cols.to_be_bytes()).unwrap();
    
    // Pixels (2x2)
    let pixels: [u8; 4] = [0, 127, 255, 64];
    f.write_all(&pixels).unwrap();
    f.flush().unwrap();
    
    // Test parser
    let images = neuromorphic_engine::mnist::load_mnist_images(temp_path.to_str().unwrap()).unwrap();
    
    assert_eq!(images.len(), 1);
    let img = &images[0];
    assert_eq!(img.len(), 4);
    
    assert_eq!(img[0], 0.0);
    assert!((img[1] - (127.0 / 255.0)).abs() < 0.01);
    assert_eq!(img[2], 1.0);
    assert!((img[3] - (64.0 / 255.0)).abs() < 0.01);
    
    let _ = std::fs::remove_file(temp_path);
}

// --- Test 2: Retina Topology ---
#[test]
fn test_retina_topology() {
    let graph = neuromorphic_engine::runner::modes::setup_mnist_network();
    
    // Count photoreceptors
    let photoreceptors: Vec<_> = graph.neurons.iter().filter(|n| n.type_ == neuromorphic_engine::engine::ffi::NeuronType::Photoreceptor as u8).collect();
    
    assert_eq!(photoreceptors.len(), 784, "Should be exactly 28x28 (784) Photoreceptors");
    
    // Verify they have outgoing synapses
    let mut has_synapses = false;
    for s in &graph.synapses {
        if s.source_id < 784 {
            has_synapses = true;
            break;
        }
    }
    
    assert!(has_synapses, "Photoreceptors must have outgoing synapses to the rest of the network");
}

// --- Test 3: Phototransduction ---
#[test]
fn test_retina_stimulation() {
    let mut graph = neuromorphic_engine::runner::modes::setup_mnist_network();
    
    // Set 10 white, 10 dark
    for i in 0..10 {
        graph.neurons[i].external_input = 1.0;
        graph.neurons[i + 10].external_input = 0.0;
    }
    
    let env = Environment {
        global_oxygen_level: 1.0,
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0,
        toxin_levels: 0.0,
        light_level: 1.0,
        stdp_learning_rate: 0.1,
        wakefulness_multiplier: 1.0,
            thermal_noise_scale: 1.0,
    };
    
    // Pre-tick voltage check
    let _initial_v = graph.neurons[0].current_voltage;
    
    for tick in 1..=10 {
        graph.tick(tick, env);
    }
    
    let white_v = graph.neurons[0].current_voltage;
    let dark_v = graph.neurons[10].current_voltage;
    
    println!("White pixel v: {}, Dark pixel v: {}", white_v, dark_v);
    
    assert!(white_v < dark_v, "White pixels should be more hyperpolarized than dark pixels");
}
