use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use neuromorphic_engine::graph::NetworkGraph;
use neuromorphic_engine::ffi::{BaseNeuron, Synapse, Environment, NeuronType, Compartment};

fn setup_benchmark_graph(num_neurons: usize, num_synapses: usize) -> NetworkGraph {
    let mut graph = NetworkGraph::new();
    for i in 0..num_neurons {
        let mut n = BaseNeuron {
            id: i as u32,
            type_: NeuronType::Pyramidal as u8,
            death_reason: 0,
            is_alive: true,
            _pad1: 0,
            size: 1.0,
            current_voltage: -70.0,
            activation_threshold: -55.0,
            resting_voltage: -70.0,
            energy_atp: 100.0,
            max_atp: 100.0,
            leak_coeff: 0.1,
            calcium_level: 0.0,
            vesicles_pool: 100.0,
            refractory_time_left: 0.0,
            last_spike_tick: 0,
            spikes_in_last_window: 0,
            metabolic_wear: 0,
            x: (i % 100) as u16,
            y: (i / 100 % 100) as u16,
            z: (i / 10000) as u16,
            _pad2: 0,
            branches: [Compartment { branch_type: 0, refractory_timer: 0, voltage: 0, nmda_threshold: 0, _padding: [0; 2] }; 8],
        };
        if i % 3 == 0 {
            n.is_alive = false;
        }
        graph.neurons.push(n);
    }
    
    for i in 0..num_synapses {
        graph.synapses.push(Synapse {
            source_id: (i % num_neurons) as u32,
            target_id: ((i + 1) % num_neurons) as u32,
            weight: 1.0,
            neurotransmitter_buffer: 5.0,
            last_activation_tick: 0,
            ticks_below_threshold: 0,
            ticks_until_delivery: 0,
            flags: 0,
            _padding: [0; 2],
        });
    }
    graph
}

fn bench_astrocyte(c: &mut Criterion) {
    let mut graph = setup_benchmark_graph(100_000, 1_000_000);
    c.bench_function("astrocyte_100k_1M", |b| b.iter(|| {
        graph.run_astrocyte_service(black_box(10));
    }));
}

fn bench_microglia(c: &mut Criterion) {
    let graph = setup_benchmark_graph(100_000, 1_000_000);
    c.bench_function("microglia_100k_1M", |b| {
        b.iter_batched(
            || graph.clone(),
            |mut g| g.run_microglia_service(black_box(10)),
            criterion::BatchSize::LargeInput,
        )
    });
}

fn bench_neurogenesis(c: &mut Criterion) {
    let mut graph = setup_benchmark_graph(100_000, 1_000_000);
    c.bench_function("neurogenesis_100k_1M", |b| b.iter(|| {
        graph.run_neurogenesis_service(black_box(500));
    }));
}

fn bench_tick_ffi(c: &mut Criterion) {
    let mut graph = setup_benchmark_graph(100_000, 1_000_000);
    let env = Environment {
        global_oxygen_level: 1.0,
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0,
        toxin_levels: 0.0,
        light_level: 0.0,
        seed: 42,
    };
    c.bench_function("tick_ffi_100k_1M", |b| b.iter(|| {
        graph.tick(black_box(10), black_box(env.clone()));
    }));
}

criterion_group!(benches, bench_astrocyte, bench_microglia, bench_neurogenesis, bench_tick_ffi);
criterion_main!(benches);
