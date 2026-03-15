# Neuromorphic Engine: Visualization & Metrics

Guide to using the analysis, testing, and real-time visualization tools.

## CLI Commands

The engine supports several testing and demonstration modes, callable via standard Cargo arguments:

| Command | Description | Primary Result |
| --------- | ---------- | ------------------- |
| `cargo run --release` | Default headless simulation | Streams data; exports `reports/simulation_report.json` |
| `cargo run --release -- --fresh` | Delete old save and regenerate network | Generates new `brain_save.bin` and runs headless |
| `cargo run --release -- --ui` | Runs the simulation with the WebSocket HUD | Streams JSON to `visualizer/index.html` |
| `cargo run --release -- --benchmark` | 10k neurons / 100k synapses TPS load test | `graphs/benchmark_load.png` |
| `cargo run --release -- --stress-test` | Epilepsy and hypoxia test | `graphs/stress_test_results.png` |
| `cargo run --release -- --dopamine-rush` | Learning under dopamine rush | `graphs/dopamine_rush_results.png` |

> **Note:** Appending `--neurons N` and `--synapses M` along with `--fresh` allows for testing networks of any custom scale.

## Real-Time UI HUD (`visualizer/`)

The system includes a vanilla HTML/JS visualizer running via WebSockets (`--ui` flag). 
Open `visualizer/index.html` in any browser to monitor:
- **Neural Graph:** Live scatter plot of neuron positions.
- **ATP Heatmaps:** Color-coded energy levels (Red = Critical, Green = Healthy).
- **Homeostasis Bars:** Real-time metrics for Global Oxygen, Glucose, and Environmental Ticks Per Second (TPS).

## Interpreting Offline Charts (`plotters`)

### 1. Activity & Biology (Benchmark)

- **Top Panel:** Spike histogram. Shows overall network phase resonance.
- **Middle Panel:** ATP vs Calcium. If ATP falls below 30%, it indicates high stress. Calcium growth above 80 triggers apoptosis (Phoenix Loop).
- **Bottom Panel:** Processing time (FFI vs Rust). Useful for identifying parallelization bottlenecks in the `BaseNeuron` array.

### 2. Stress Test Analysis

- Observes the behavior of the `EnvironmentRegulator` dropping `global_oxygen_level` in response to hardware strain.
- Visualizes metabolic shutdown events and the subsequent revival of nodes.

### 3. Plasticity (Dopamine Rush)

- Shows the correlation between global dopamine multipliers and precise `weight` growth.
- Demonstrates structural saturation: weights cap out strictly at `MAX_SYNAPSE_WEIGHT`, avoiding numerical explosions.

## JSON Telemetry Reports

Whenever the engine concludes a long-running mode, it exports a detailed payload into the `reports/` directory (e.g., `simulation_report.json`).

These reports contain:
- `simulation_summary`: Sim speed (TPS) and final synapse counts (useful for evaluating Microglia pruning).
- `incident_log`: How many neurons died due to ATP starvation vs Calcium toxicity.
- `population_distributions`: Binned histograms of Synaptic Weights and ATP levels across the entire graph.

## Adding New Metrics

To add a new metric to the pipeline:

1. **Telemetry:** Add a tracking variable inside the `Telemetry` struct (`src/monitor/telemetry.rs`).
2. **Aggregation:** Update the corresponding variable in the `update()` tick loop.
3. **Export:** For JSON, serialize it in `export_simulation_report`. For charts, add a new plotting primitive in `src/monitor/visualization.rs` via the `plotters` library.
