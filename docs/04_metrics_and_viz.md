# Neuromorphic Engine: Visualization & Metrics

Guide to using analysis and visualization tools.

## CLI Commands

The engine supports several testing and demonstration modes:

| Command | Description | Primary Result |
|---------|----------|-------------------|
| `cargo run --release` | Base Spec 8 simulation | `graphs/phase8_sensors.png` |
| `cargo run --release -- --benchmark` | 10k neurons / 100k synapses test | `graphs/benchmark_load.png` |
| `cargo run --release -- --stress-test` | Epilepsy and hypoxia test | `graphs/stress_test_results.png` |
| `cargo run --release -- --dopamine-rush` | Learning under dopamine rush | `graphs/dopamine_rush_results.png` |

## Interpreting Charts

### 1. Activity & Biology (Benchmark)
- **Top Panel:** Spike histogram. Shows overall network activity.
- **Middle Panel:** ATP vs Calcium. If ATP falls below 30%, it's a warning sign. Calcium growth above 80 leads to mortality risk.
- **Bottom Panel:** Processing time (FFI vs Rust). Allows identifying parallelization bottlenecks.

### 2. Stress Test Analysis
- Allows seeing the moment of neuron death.
- Studies the influence of `global_oxygen_level` on the survival curve.
- Visualizes `metabolic_wear` growth at high spike frequencies.

### 3. Plasticity (Dopamine Rush)
- Shows the correlation between dopamine level and synapse `weight` growth.
- Demonstrates saturation: weight cannot grow indefinitely, hitting biological limits.

## Adding New Metrics
To add a metric:
1. Add a field to the data structure in `main.rs` (e.g., `BenchmarkData`).
2. Calculate the required value (average or sum) in the tick loop.
3. Update the corresponding `render_..._chart` function using the `plotters` library.
