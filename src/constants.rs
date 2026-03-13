// src/constants.rs
// Centralized registry for all neuromorphic engine constants
#![allow(dead_code)]

// --- Simulation Timing & Intervals ---
pub const SERVICE_TICK_INTERVAL: u32 = 10;
pub const NEUROGENESIS_INTERVAL: u32 = 500;
pub const TELEMETRY_PRINT_INTERVAL: u32 = 500;
pub const METRICS_COLLECTION_INTERVAL: u32 = 50;
pub const WINDOW_SIZE: u32 = 20;

pub const WS_SNAPSHOT_INTERVAL_BENCHMARK: u32 = 15;
pub const WS_SNAPSHOT_INTERVAL_SIMULATION: u32 = 15;
pub const VISUAL_SPIKE_PERSISTENCE_TICKS: u32 = 25;

pub const DEFAULT_SIMULATION_TICKS: u32 = 5000;
pub const DEFAULT_BENCHMARK_TICKS: u32 = 50_000;
pub const DEFAULT_BENCHMARK_NEURONS: usize = 1000;
pub const DEFAULT_BENCHMARK_SYNAPSES: usize = 25_000;

// --- Stimulus & Pavlovian Conditioning ---
pub const STIMULUS_DEFAULT_INTERVAL: u32 = 1000;
pub const STIMULUS_DEFAULT_WINDOW: u32 = 50;

// --- Network Defaults ---
pub const CLUSTERING_RADIUS_SQ: f32 = 400.0; // Distance sq < 20.0^2
pub const STOCHASTIC_GROWTH_PROB: f64 = 0.01;
pub const DEFAULT_WEIGHT: f32 = 2.0;
pub const WEIGHT_SIGMA: f32 = 1.0;
pub const SEED_WEIGHT: f32 = 0.5;

pub const LONG_RANGE_PROB: f64 = 0.02;
pub const INHIBITORY_PROB: f64 = 0.1;
pub const MYELIN_PROB: f64 = 0.2;
pub const PYRAMIDAL_PROB: f64 = 0.7;

// --- Biological Thresholds & Limits (СИНХРОНИЗИРОВАНО С base_neuron.h) ---
pub const PRUNING_WEIGHT_THRESHOLD: f32 = 0.1;
pub const PRUNING_TICK_THRESHOLD: u32 = 2000;
pub const MAX_SYNAPSE_WEIGHT: f32 = 50.0; // ПОФИКСЕНО (было 30.0)
pub const MAX_SYNAPSES_TOTAL: usize = 1_000_000;
pub const MAX_SYNAPSES_PER_NEURON: usize = 2000;
pub const MAX_SYNAPSE_WEIGHT_BENCHMARK: f32 = 10.0;
pub const MIN_SYNAPSE_WEIGHT_BENCHMARK: f32 = 0.5;

pub const MAX_ATP: f32 = 100.0;
pub const EMERGENCY_ATP_THRESHOLD: f32 = 20.0; // Порог засыпания (Silence)
pub const EMERGENCY_ATP_DEATH_THRESHOLD: f32 = 0.01; // Порог смерти

pub const ASTROCYTE_ATP_SUPPLY: f32 = 10.0;
pub const ASTROCYTE_CLEAN_THRESHOLD: f32 = 50.0;
pub const ASTROCYTE_CLEAN_AMOUNT: f32 = 10.0;

// --- Physics (Mirrored from CUDA for initialization) ---
pub const NMDA_SPIKE_GAIN: f32 = 5.0; // ПЕРЕИМЕНОВАНО (было NMDA_SPIKE_THRESHOLD)
pub const RESTING_VOLTAGE: f32 = -70.0;
pub const THRESHOLD_VOLTAGE: f32 = -55.0;
pub const SPIKE_VOLTAGE: f32 = 40.0;
pub const SPIKE_DETECTION_VOLTAGE: f32 = 39.0;
pub const SUBTHRESHOLD_VOLTAGE: f32 = -40.0;
pub const VOLTAGE_STEP: f32 = 0.7;
pub const EYE_VOLTAGE_HIGH: f32 = -30.0;
pub const EYE_VOLTAGE_LOW: f32 = -70.0;
pub const SOMA_VOLTAGE_MIN: f32 = -90.0;
pub const SOMA_VOLTAGE_MAX: f32 = 60.0;

// --- Environment Defaults ---
pub const DEFAULT_OXYGEN: f32 = 1.0;
pub const HYPOXIC_OXYGEN: f32 = 0.01;
pub const DEFAULT_GLUCOSE: f32 = 1.0;
pub const DEFAULT_DOPAMINE: f32 = 1.0;
pub const LOW_DOPAMINE: f32 = 0.1;
pub const HIGH_DOPAMINE: f32 = 5.0;
pub const BOOSTED_DOPAMINE: f32 = 4.0;
pub const EMERGENCE_DOPAMINE: f32 = 5.0;
pub const MORPHOLOGY_DOPAMINE: f32 = 2.0;
pub const DEFAULT_TOXINS: f32 = 0.0;
pub const DEFAULT_LIGHT: f32 = 0.0;
pub const LIGHT_HIGH: f32 = 0.8;
pub const LIGHT_LOW: f32 = 0.1;

// --- Visualization Defaults ---
pub const CANVAS_WIDTH: u32 = 1200;
pub const CANVAS_HEIGHT_XL: u32 = 1000;
pub const CANVAS_HEIGHT_LG: u32 = 800;
pub const CHART_MARGIN: u32 = 10;
pub const LABEL_AREA_SIZE: u32 = 40;
pub const CAPTION_FONT_SIZE: u32 = 20;
pub const TICK_LIMIT_CHART: f32 = 1000.0;
pub const METRIC_MIN_F32: f32 = 0.0;
pub const METRIC_MAX_F32: f32 = 100.0;
pub const VOLTAGE_MIN_F32: f32 = -90.0;
pub const VOLTAGE_MAX_F32: f32 = 60.0; // ПОФИКСЕНО (чтобы графики не обрезали пики)
pub const BUFFER_MAX_F32: f32 = 20.0;
pub const WEIGHT_MAX_F32: f32 = 50.0; // ПОФИКСЕНО (совпадает с MAX_SYNAPSE_WEIGHT)
pub const DOPAMINE_MAX_F32: f32 = 8.0; // ПОФИКСЕНО (под твой хардкорный буст из stimulus.rs)
pub const EMERGENCE_TIME_LIMIT: f32 = 2000.0;
pub const MORPHOLOGY_TIME_LIMIT: f32 = 3000.0;
pub const REFLEX_TIME_LIMIT: f32 = 1000.0;
pub const INHIBITION_TIME_LIMIT: f32 = 1000.0;
pub const PLASTICITY_TIME_LIMIT: f32 = 1000.0;

// --- Random Seeds (Phase 39 Cleanup) ---
pub const SEED_BENCHMARK: u64 = 42;
pub const SEED_SIMULATION: u64 = 1337;
pub const SEED_INHIBITION: u64 = 999;
pub const SEED_REFLEX: u64 = 888;
pub const SEED_STRESS: u64 = 666;
pub const SEED_DOPAMINE: u64 = 777;
pub const SEED_EMERGENCE: u64 = 1234;

// --- Telemetry & Analytics ---
pub const SECONDS_PER_TICK: f64 = 0.001; // 1 tick = 1ms of biological time
pub const WEIGHT_BINS_COUNT: usize = 10;
pub const ATP_BINS_COUNT: usize = 5;
pub const MAX_U16_AS_F32: f32 = 65535.0; // Normalization constant for 16-bit fixed-point metrics
pub const FATIGUE_SLEEP_THRESHOLD: f32 = 5000.0; // Metabolic wear limit to trigger Sleep Cycle

// --- Constants from Phase 34 Tests ---
pub const VOLTAGE_10_PACKED: u16 = 11000;  // (10.0 + 100.0) * 100.0
pub const VOLTAGE_50_PACKED: u16 = 15000;  // (50.0 + 100.0) * 100.0
pub const VOLTAGE_60_PACKED: u16 = 16000;  // (60.0 + 100.0) * 100.0

// --- Data Packing & Geometry ---
pub const MAX_COORD: f32 = 100.0; // Spatial boundary size (100x100x100)
pub const COORD_SCALE: f32 = 655.35; // 65535.0 / 100.0 (maps 0.0..100.0 to 0..65535)
pub const ZERO_VOLTAGE_PACKED: u16 = 10000; // (0.0 + 100.0) * 100.0
pub const NMDA_THRESHOLD_PACKED: u16 = 10500; // (5.0 + 100.0) * 100.0