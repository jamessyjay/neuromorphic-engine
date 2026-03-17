export const nodeMap = new Map();
export const activePulses = [];
export const layerVisibility = { 1: true, 2: true, 4: true, 5: true, 6: true };

export let neuronMeshes = [];
export let synapseLines = null;
export let pulseSystem = null;
export let colorMode = 'type';
export let selectedNeuronId = null;
export let currentEnv = { dopamine: 1.0, lr: 0.1 };

// Setters
export function setNeuronMeshes(meshes) { neuronMeshes = meshes; }
export function setSynapseLines(lines) { synapseLines = lines; }
export function setPulseSystem(system) { pulseSystem = system; }
export function setColorMode(mode) { colorMode = mode; }
export function setSelectedNeuronId(id) { selectedNeuronId = id; }
export function setCurrentEnv(env) { currentEnv = env; }
