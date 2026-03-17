export const nodeMap = new Map();
export const activePulses = [];
export const currentEnv = { dopamine: 1.0, lr: 0.1 };
export let neuronMeshes = [];
export let synapseLines = null;
export let pulseSystem = null;
export let selectedNeuronId = null;

export let colorMode = 'type'; 

export const layerVisibility = {
    6: true, 
    5: true, 
    4: true, 
    2: true, 
    1: true  
};

// Classifier UI State
export let smoothedPredictions = new Array(10).fill(0);
export let lastDisplayedWinner = -1;
export let winnerHoldTicks = 0;

export function setNeuronMeshes(meshes) { neuronMeshes = meshes; }
export function setSynapseLines(lines) { synapseLines = lines; }
export function setPulseSystem(sys) { pulseSystem = sys; }
export function setColorMode(mode) { colorMode = mode; }
export function setSelectedNeuronId(id) { selectedNeuronId = id; }
export function setCurrentEnv(env) { currentEnv = env; }
export function setLastDisplayedWinner(w) { lastDisplayedWinner = w; }
export function setWinnerHoldTicks(t) { winnerHoldTicks = t; }
