import * as THREE from 'three';

export const FLAG_IS_INHIBITED = 1;
export const FLAG_IS_PREDICTIVE = 2;

export const TYPE_COLORS = {
    0: new THREE.Color(0x00aaff), 1: new THREE.Color(0x44ff44),
    2: new THREE.Color(0xffaa00), 3: new THREE.Color(0xff00ff),
    4: new THREE.Color(0xff4444), 5: new THREE.Color(0xffff00),
    6: new THREE.Color(0x00ffff), 
    dead_calcium: new THREE.Color(0xff2222), // Red for calcium overload
    dead_atp: new THREE.Color(0x2222ff),     // Blue for hypoxia/starvation
    dead_regen: new THREE.Color(0x22ff22),   // Green for regenerating (Phoenix)
    dead: new THREE.Color(0x220000)
};

export const MAX_PULSES = 3000;
export const PULSE_SPEED = 0.015;
export const VISUAL_SAMPLING = 0.3;
