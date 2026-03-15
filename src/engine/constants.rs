// src/constants.rs
// Centralized registry for all neuromorphic engine constants
#![allow(dead_code)]

// --- Simulation Timing & Intervals ---
/// Interval (in ticks) between periodic maintenance tasks (e.g., global syncs)
pub const SERVICE_TICK_INTERVAL: u32 = 10;
/// Interval (in ticks) at which neurogenesis (spawning new neurons/synapses) is evaluated
pub const NEUROGENESIS_INTERVAL: u32 = 500;
/// Interval (in ticks) for printing telemetry/status updates to the console
pub const TELEMETRY_PRINT_INTERVAL: u32 = 500;
/// Interval (in ticks) for collecting internal metrics (for graphs and analytics)
pub const METRICS_COLLECTION_INTERVAL: u32 = 50;
/// Sliding window size (in ticks) used for calculating moving averages (e.g., spike rates)
pub const WINDOW_SIZE: u32 = 20;

/// Ticks between WebSocket frame broadcasts during fast benchmark modes
pub const WS_SNAPSHOT_INTERVAL_BENCHMARK: u32 = 15;
/// Ticks between WebSocket frame broadcasts during normal simulation mode
pub const WS_SNAPSHOT_INTERVAL_SIMULATION: u32 = 15;
/// Interval to re-broadcast initial state so late clients can sync
pub const WS_INITIAL_STATE_INTERVAL: u32 = 5000;
/// Delay in milliseconds between UI updates to throttle at ~60fps
pub const WS_THROTTLE_MS: u64 = 16;
/// How many ticks a spike visually lingers on the UI before fading out completely
pub const VISUAL_SPIKE_PERSISTENCE_TICKS: u32 = 25;
/// Interval in ticks when we should yield the async executor
pub const ASYNC_YIELD_INTERVAL: u32 = 50;

/// Default number of ticks to run a standard headless simulation if not specified
pub const DEFAULT_SIMULATION_TICKS: u32 = 5000;
/// Default number of ticks for the benchmark mode if not overridden
pub const DEFAULT_BENCHMARK_TICKS: u32 = 50_000;
/// Default network size (neurons) for benchmarks
pub const DEFAULT_BENCHMARK_NEURONS: usize = 1000;
/// Default network size (synapses) for benchmarks
pub const DEFAULT_BENCHMARK_SYNAPSES: usize = 25_000;
/// Ticks during which the network receives nursery protection (ATP minimum)
pub const NURSERY_PROTECTION_TICKS: u32 = 5000;

// --- Stimulus & Pavlovian Conditioning ---
/// Default tick interval between automated stimulus injections
pub const STIMULUS_DEFAULT_INTERVAL: u32 = 1000;
/// Default window size (in ticks) during which a stimulus remains active
pub const STIMULUS_DEFAULT_WINDOW: u32 = 50;

// --- Network Defaults ---
/// Maximum squared spatial distance for preferred local synapse connections
pub const CLUSTERING_RADIUS_SQ: f32 = 400.0; // Distance sq < 20.0^2
/// Probability of random spontaneous synapse formation during neurogenesis
pub const STOCHASTIC_GROWTH_PROB: f64 = 0.01;
/// Default starting weight for a newly spawned synapse
pub const DEFAULT_WEIGHT: f32 = 2.0;
/// Standard deviation for Gaussian randomization of starting synapse weights
pub const WEIGHT_SIGMA: f32 = 1.0;
/// Baseline structural weight given to seed/initial connections
pub const SEED_WEIGHT: f32 = 0.5;

/// Probability of a synapse ignoring spatial clustering and forming a long-range connection
pub const LONG_RANGE_PROB: f64 = 0.02;
/// Probability that a newly created neuron will be inhibitory (GABAergic)
pub const INHIBITORY_PROB: f64 = 0.1;
/// Probability that a new axon/synapse will be myelinated (faster transmission)
pub const MYELIN_PROB: f64 = 0.2;
/// Probability that a newly created excitatory neuron will be Pyramidal type
pub const PYRAMIDAL_PROB: f64 = 0.7;

// --- Biological Thresholds & Limits (SYNCHRONIZED WITH base_neuron.h) ---
/// If a synapse's weight falls below this during sleep/cleanup, it gets pruned
pub const PRUNING_WEIGHT_THRESHOLD: f32 = 0.1;
/// If a synapse hasn't been active for this many ticks, it becomes a candidate for pruning
pub const PRUNING_TICK_THRESHOLD: u32 = 2000;
/// Absolute maximum weight any synapse can reach (Hard limit)
pub const MAX_SYNAPSE_WEIGHT: f32 = 50.0; // FIXED (was 30.0)
/// Engine hard limit on total number of synapses across the whole graph
pub const MAX_SYNAPSES_TOTAL: usize = 1_000_000;
/// Engine hard limit on maximum outgoing synapses from a single neuron
pub const MAX_SYNAPSES_PER_NEURON: usize = 2000;
/// Maximum synapse weight specifically enforced during certain benchmark tests
pub const MAX_SYNAPSE_WEIGHT_BENCHMARK: f32 = 10.0;
/// Minimum synapse weight specifically enforced during certain benchmark tests
pub const MIN_SYNAPSE_WEIGHT_BENCHMARK: f32 = 0.5;

/// Maximum metabolic energy (ATP) capacity for a healthy neuron
pub const MAX_ATP: f32 = 100.0;
/// ATP level below which a neuron enters a quiet 'sleep' state to recover
pub const EMERGENCY_ATP_THRESHOLD: f32 = 20.0; // Sleep threshold (Silence)
/// ATP level below which a neuron permanently dies (apoptosis)
pub const EMERGENCY_ATP_DEATH_THRESHOLD: f32 = 0.01; // Death threshold

/// Amount of ATP provided by an astrocyte to a neuron per recovery cycle
pub const ASTROCYTE_ATP_SUPPLY: f32 = 10.0;
/// Toxicity level at which astrocytes actively begin cleaning the local environment
pub const ASTROCYTE_CLEAN_THRESHOLD: f32 = 50.0;
/// Amount of toxicity removed by astrocytes per cleaning cycle
pub const ASTROCYTE_CLEAN_AMOUNT: f32 = 10.0;

// --- Physics (Mirrored from CUDA for initialization) ---
/// Voltage multiplier/gain applied when an NMDA dendritic spike triggers
pub const NMDA_SPIKE_GAIN: f32 = 5.0; // RENAMED (was NMDA_SPIKE_THRESHOLD)
/// Baseline resting membrane potential (mV) of a standard neuron
pub const RESTING_VOLTAGE: f32 = -70.0;
/// Membrane potential (mV) required to trigger an Action Potential
pub const THRESHOLD_VOLTAGE: f32 = -55.0;
/// Peak voltage (mV) reached during an Action Potential
pub const SPIKE_VOLTAGE: f32 = 40.0;
/// Voltage threshold used by the UI/Metrics to register that a spike occurred
pub const SPIKE_DETECTION_VOLTAGE: f32 = 39.0;
/// High depolarization state just below firing (used for logic branching)
pub const SUBTHRESHOLD_VOLTAGE: f32 = -40.0;
/// Rate of voltage decay/leak back toward resting potential per tick
pub const VOLTAGE_STEP: f32 = 0.7;
/// Peak voltage generated by Photoreceptor neurons in bright light
pub const EYE_VOLTAGE_HIGH: f32 = -30.0;
/// Baseline voltage for Photoreceptor neurons in total darkness
pub const EYE_VOLTAGE_LOW: f32 = -70.0;
/// Absolute minimum hyperpolarization limit for the soma
pub const SOMA_VOLTAGE_MIN: f32 = -90.0;
/// Absolute maximum depolarization limit for the soma
pub const SOMA_VOLTAGE_MAX: f32 = 60.0;

// --- Environment Defaults ---
/// Baseline global oxygen level in the simulation environment
pub const DEFAULT_OXYGEN: f32 = 1.0;
/// Oxygen level considered critical hypoxia, triggering massive cellular shutdown
pub const HYPOXIC_OXYGEN: f32 = 0.01;
/// Baseline global glucose (energy source) level
pub const DEFAULT_GLUCOSE: f32 = 1.0;
/// Baseline dopamine multiplier (1.0 = neutral STDP learning rate)
pub const DEFAULT_DOPAMINE: f32 = 1.0;
/// Depressed dopamine state (suppresses learning and plasticity)
pub const LOW_DOPAMINE: f32 = 0.1;
/// Highly elevated dopamine state (accelerates STDP and reinforcement)
pub const HIGH_DOPAMINE: f32 = 5.0;
/// Moderately elevated dopamine for specific reward events
pub const BOOSTED_DOPAMINE: f32 = 4.0;
/// Specific dopamine level injected during Emergence tests
pub const EMERGENCE_DOPAMINE: f32 = 5.0;
/// Dopamine level required to encourage structural morphology changes
pub const MORPHOLOGY_DOPAMINE: f32 = 2.0;
/// Baseline environmental toxicity (starts clean)
pub const DEFAULT_TOXINS: f32 = 0.0;
/// Baseline ambient light level (darkness)
pub const DEFAULT_LIGHT: f32 = 0.0;
/// High ambient light level (triggers photoreceptors strongly)
pub const LIGHT_HIGH: f32 = 0.8;
/// Dim ambient light level
pub const LIGHT_LOW: f32 = 0.1;

// --- Visualization Defaults ---
/// Default width for rendered metric charts
pub const CANVAS_WIDTH: u32 = 1200;
/// Extra large height for complex multi-panel charts
pub const CANVAS_HEIGHT_XL: u32 = 1000;
/// Large height for standard multi-panel charts
pub const CANVAS_HEIGHT_LG: u32 = 800;
/// Outer margin in pixels for rendered charts
pub const CHART_MARGIN: u32 = 10;
/// Space reserved for X/Y axis labels in charts
pub const LABEL_AREA_SIZE: u32 = 40;
/// Font size for main chart titles
pub const CAPTION_FONT_SIZE: u32 = 20;
/// Maximum ticks displayed on the X-axis for short timescale charts
pub const TICK_LIMIT_CHART: f32 = 1000.0;
/// Universal minimum Y-axis bound for percentage metrics (e.g., ATP)
pub const METRIC_MIN_F32: f32 = 0.0;
/// Universal maximum Y-axis bound for percentage metrics (e.g., ATP)
pub const METRIC_MAX_F32: f32 = 100.0;
/// Y-axis minimum bound for plotting membrane voltage
pub const VOLTAGE_MIN_F32: f32 = -90.0;
/// Y-axis maximum bound for plotting membrane voltage
pub const VOLTAGE_MAX_F32: f32 = 60.0; // FIXED (so charts don't clip peaks)
/// Y-axis bound for plotting neurotransmitter buffer levels
pub const BUFFER_MAX_F32: f32 = 20.0;
/// Y-axis bound for plotting synaptic weights
pub const WEIGHT_MAX_F32: f32 = 50.0; // FIXED (matches MAX_SYNAPSE_WEIGHT)
/// Y-axis bound for plotting global dopamine levels
pub const DOPAMINE_MAX_F32: f32 = 8.0; // FIXED (for hardcore boost from stimulus.rs)
/// Time limit for testing emergent behaviors
pub const EMERGENCE_TIME_LIMIT: f32 = 2000.0;
/// Time limit for testing structural morphology changes
pub const MORPHOLOGY_TIME_LIMIT: f32 = 3000.0;
/// Time limit for testing reflex arcs
pub const REFLEX_TIME_LIMIT: f32 = 1000.0;
/// Time limit for testing inhibitory network effects
pub const INHIBITION_TIME_LIMIT: f32 = 1000.0;
/// Time limit for testing synaptic plasticity (STDP)
pub const PLASTICITY_TIME_LIMIT: f32 = 1000.0;

// --- Random Seeds (Phase 39 Cleanup) ---
/// RNG seed used for reproducible benchmarks
pub const SEED_BENCHMARK: u64 = 42;
/// RNG seed used for standard predictable simulations
pub const SEED_SIMULATION: u64 = 1337;
/// RNG seed used for inhibition tests
pub const SEED_INHIBITION: u64 = 999;
/// RNG seed used for reflex tests
pub const SEED_REFLEX: u64 = 888;
/// RNG seed used for network stress tests
pub const SEED_STRESS: u64 = 666;
/// RNG seed used for dopamine modulation tests
pub const SEED_DOPAMINE: u64 = 777;
/// RNG seed used for emergence complexity tests
pub const SEED_EMERGENCE: u64 = 1234;

// --- Telemetry & Analytics ---
pub const SECONDS_PER_TICK: f64 = 0.001; // 1 tick = 1ms of biological time
pub const WEIGHT_BINS_COUNT: usize = 10;
pub const ATP_BINS_COUNT: usize = 5;
pub const MAX_U16_AS_F32: f32 = 65535.0; // Normalization constant for 16-bit fixed-point metrics
pub const FATIGUE_SLEEP_THRESHOLD: f32 = 60_000.0; // Metabolic wear limit to trigger Sleep Cycle

// --- Constants from Phase 34 Tests ---
pub const VOLTAGE_10_PACKED: u16 = 11000;  // (10.0 + 100.0) * 100.0
pub const VOLTAGE_50_PACKED: u16 = 15000;  // (50.0 + 100.0) * 100.0
pub const VOLTAGE_60_PACKED: u16 = 16000;  // (60.0 + 100.0) * 100.0

// --- Data Packing & Geometry ---
pub const MAX_COORD: f32 = 100.0; // Spatial boundary size (100x100x100)
pub const COORD_SCALE: f32 = 655.35; // 65535.0 / 100.0 (maps 0.0..100.0 to 0..65535)
pub const ZERO_VOLTAGE_PACKED: u16 = 10000; // (0.0 + 100.0) * 100.0
pub const NMDA_THRESHOLD_PACKED: u16 = 10500; // (5.0 + 100.0) * 100.0
// --- Modes & UI Testing Constants ---

/// Number of ticks to wait before ending the UI visualizer test (300 seconds * 10 ticks/sec roughly)
pub const UI_TEST_DURATION_SECS: u64 = 300;
/// Center coordinate for testing UI topologies (maps to 50 in world space)
pub const UI_COORD_CENTER: u16 = 32768;
/// Right edge coordinate for testing UI topologies (maps to 91 in world space)
pub const UI_COORD_RIGHT: u16 = 60000;
/// Left edge coordinate for testing UI topologies (maps to 7 in world space)
pub const UI_COORD_LEFT: u16 = 5000;
/// Synapse weight for the UI test mode
pub const UI_TEST_SYNAPSE_WEIGHT: f32 = 50.0;
/// Voltage level for artificially forcing a neuron to spike
pub const FORCED_SPIKE_VOLTAGE: f32 = 40.0;
/// Base ATP level to set when artificially resetting a neuron
pub const FULL_ATP_LEVEL: f32 = 100.0;

// --- Stimulus & Conditioning Constants ---
/// Percentage of the network dedicated to sensory input (Bell + Meat)
pub const SENSOR_GROUP_PERCENTAGE: f32 = 0.04;
/// The ideal time window (in ticks/ms) between stimulus A (Bell) and B (Meat) for optimal STDP learning
pub const STDP_IDEAL_WINDOW_TICKS: u32 = 30;
/// Multiplier applied to dopamine when the reward (Meat) is presented
pub const REWARD_DOPAMINE_MULTIPLIER: f32 = 5.0;
/// Rate at which dopamine decays back to the baseline of 1.0
pub const DOPAMINE_DECAY_RATE: f32 = 0.1;
/// Number of training cycles before doing a test cycle (only bell, no meat)
pub const TRAINING_CYCLES_BEFORE_TEST: u32 = 5;

// --- Metrics & Data Scaling Constants ---
/// ATP percentage threshold below which the neuron visually indicates low energy in UI
pub const VISUAL_LOW_ATP_THRESHOLD: f32 = 25.0;
/// Minimum ATP maintained by the nursery protection mechanism during startup ticks
pub const NURSERY_MIN_ATP: f32 = 50.0;
/// Conversion factor for seconds to milliseconds
pub const SECONDS_TO_MS: f32 = 1000.0;
/// Divisor for UI sampling intervals based on total elements
pub const UI_SAMPLING_DIVISOR: f32 = 1000.0;
/// Convert Bytes to Kilobytes
pub const BYTES_TO_KB: f32 = 1024.0;
/// Hardcoded interval at which we forcefully broadcast full synapse array to the UI
pub const WS_SYNAPSE_FORCE_SYNC_INTERVAL: u32 = 500;

// --- Network Generation Proportions ---
/// Proportion of sensors in auto-generated networks
pub const SENSOR_PROPORTION: f32 = 0.04;
/// Proportion of motors in auto-generated networks
pub const MOTOR_PROPORTION: f32 = 0.04;
/// Proportion of pacemakers in auto-generated networks
pub const PACEMAKER_PROPORTION: f32 = 0.02;
/// Proportion of interneurons in auto-generated networks
pub const INTERNEURON_PROPORTION: f32 = 0.15;

// --- Physical Bounds & Constants ---
/// Max bounds for coordinates mapped to UI Space 
pub const COORD_MAX: u16 = 65535;
/// Multiplier used to adjust the spatial clustering radius logic
pub const SPATIAL_RADIUS_MULTIPLIER: f32 = 0.15;
/// Scale modifier for base initial synapse weights
pub const BASE_WEIGHT_MODIFIER: f32 = 2.0;
/// The maximum number of dendritic branches available on a Pyramidal neuron
pub const DENDRITIC_BRANCH_COUNT: u8 = 8;
/// A standard semi-depolarized state voltage used to seed networks to avoid immediate flatlines
pub const PRE_SEED_VOLTAGE: f32 = -30.0;

// --- Benchmark Mode Default Arguments ---
/// Number of ticks for scaling mode tests
pub const SCALING_BENCHMARK_TICKS: u32 = 500;
/// Density multiplier: how many synapses per neuron we request in the benchmark
pub const SCALING_SYNAPSE_DENSITY: usize = 10;

// --- Connection Default Weights ---
/// Synapse weight connecting an external eye/sensor to the first brain layer
pub const WEIGHT_EYE_TO_BRAIN: f32 = 15.0;
/// Synapse weight connecting the brain output to a motor/muscle node
pub const WEIGHT_BRAIN_TO_MUSCLE: f32 = 12.0;
/// Heavy synapse weight used to strongly bias network tests
pub const WEIGHT_STRONG_BIAS: f32 = 10.0;
/// Base testing connection weight
pub const WEIGHT_TEST_DEFAULT: f32 = 5.0;
/// Injected voltage step size during manual test ticks
pub const VOLTAGE_INJECTION_STEP: f32 = 15.0;
/// Supercharged voltage injection for emergence testing
pub const VOLTAGE_EMERGENCE_INJECTION: f32 = 17.5;
/// High voltage used to force a neuron to spike
pub const VOLTAGE_FORCED_SPIKE: f32 = 20.0;
/// Number of simulated test ticks
pub const TOTAL_TEST_TICKS: u32 = 3000;
/// Number of simulated long test ticks
pub const TOTAL_LONG_TEST_TICKS: u32 = 50000;
/// Hypoxic trigger for oxygen deprivation tests
pub const HYPOXIA_TICK_TRIGGER: u32 = 300;

// --- Light Cycle Testing Constants ---
/// The duration (in ticks) of one full light/dark phase in reflex testing
pub const LIGHT_CYCLE_DURATION: u32 = 50;

// --- Test Durations ---
/// Duration for medium length testing scenarios
pub const MEDIUM_TEST_TICKS: u32 = 2000;
/// Duration for long running test scenarios (e.g. stress tests)
pub const LONG_TEST_TICKS: u32 = 5000;

// --- Dopamine Rush Test Intervals ---
/// End tick for the low dopamine phase
pub const DOPAMINE_PHASE_1_END: u32 = 200;
/// End tick for the high dopamine phase
pub const DOPAMINE_PHASE_2_END: u32 = 600;
/// End tick for the recovery phase
pub const DOPAMINE_PHASE_3_END: u32 = 1000;
