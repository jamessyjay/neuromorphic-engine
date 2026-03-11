// src/constants.rs
// Centralized registry for all neuromorphic engine constants
// Some constants are reserved for future use (e.g. on Jetson Orin Nano with real CUDA).
#![allow(dead_code)]

// --- Simulation Timing & Intervals ---
pub const SERVICE_TICK_INTERVAL: u32 = 10;
pub const NEUROGENESIS_INTERVAL: u32 = 500;
pub const TELEMETRY_PRINT_INTERVAL: u32 = 500;
pub const METRICS_COLLECTION_INTERVAL: u32 = 50;
pub const WINDOW_SIZE: u32 = 20;

// --- Network Defaults ---
pub const CLUSTERING_RADIUS_SQ: f32 = 400.0; // Distance sq < 20.0^2
pub const STOCHASTIC_GROWTH_PROB: f64 = 0.2;
pub const DEFAULT_WEIGHT: f32 = 2.0;
pub const WEIGHT_SIGMA: f32 = 1.0;
pub const SEED_WEIGHT: f32 = 0.05;
pub const SEED_THICKNESS: f32 = 0.1;

// --- Biological Thresholds & Limits ---
pub const PRUNING_WEIGHT_THRESHOLD: f32 = 0.05;
pub const PRUNING_TICK_THRESHOLD: u32 = 1000;
pub const MAX_SYNAPSE_WEIGHT: f32 = 30.0;
pub const EMERGENCY_ATP_THRESHOLD: f32 = 20.0;
pub const MAX_ATP: f32 = 100.0;
pub const ASTROCYTE_ATP_SUPPLY: f32 = 10.0;
pub const ASTROCYTE_CLEAN_THRESHOLD: f32 = 50.0;
pub const ASTROCYTE_CLEAN_AMOUNT: f32 = 10.0;

// --- Visualization Defaults ---
pub const CANVAS_WIDTH: u32 = 1200;
pub const CANVAS_HEIGHT_XL: u32 = 1000;
pub const CANVAS_HEIGHT_LG: u32 = 800;
pub const CHART_MARGIN: u32 = 10;
pub const LABEL_AREA_SIZE: u32 = 40;
pub const CAPTION_FONT_SIZE: u32 = 20;
pub const TICK_LIMIT_CHART: f32 = 1000.0;

// --- Physics (Mirrored from CUDA for initialization) ---
pub const NMDA_SPIKE_THRESHOLD: f32 = 5.0;
pub const RESTING_VOLTAGE: f32 = -70.0;
pub const THRESHOLD_VOLTAGE: f32 = -55.0;
pub const SPIKE_VOLTAGE: f32 = 40.0;
