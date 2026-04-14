import { layerVisibility, colorMode, setColorMode, nodeMap, currentEnv } from './state.js';
import { FLAG_IS_PREDICTIVE, FLAG_IS_INHIBITED } from './config.js';
import { getWs } from './network.js';
import { smoothedPredictions, lastDisplayedWinner, winnerHoldTicks, setLastDisplayedWinner, setWinnerHoldTicks } from './state.js';

export let retinaCtx = null;

export function toggleLayer(layer) {
    layerVisibility[layer] = !layerVisibility[layer];
}

export function getLayerName(layerId) {
    switch(layerId) {
        case 6: return "L6 - Location";
        case 5: return "L5 - Output";
        case 4: return "L4 - Input";
        case 2: return "L2/3 - Identity";
        case 1: return "L1 - Feedback";
        default: return "Unknown";
    }
}

export function getTypeName(typeId) {
    switch(typeId) {
        case 0: return "Pyramidal";
        case 2: return "Interneuron";
        case 3: return "Pacemaker";
        case 4: return "Purkinje";
        case 5: return "Photoreceptor";
        case 6: return "Nociceptor";
        default: return "Unknown";
    }
}

export function toggleColorMode() {
    setColorMode(colorMode === 'type' ? 'layer' : 'type');
    document.getElementById('current-color-mode').innerText = colorMode.toUpperCase();
    document.getElementById('legend-type').style.display = colorMode === 'type' ? 'block' : 'none';
    document.getElementById('legend-layer').style.display = colorMode === 'layer' ? 'block' : 'none';
}

export function showPassport(node) {
    document.getElementById('neuron-passport').style.display = 'block';
    document.getElementById('np-id').innerText = node.id;
    document.getElementById('np-type').innerText = getTypeName(node.type);
    document.getElementById('np-column').innerText = node.columnId;
    document.getElementById('np-layer').innerText = getLayerName(node.layerId);
    document.getElementById('np-minicolumn').innerText = node.minicolumnId;
    updatePassportStatus(node);

    const interColumnLinks = node.outgoing.filter(syn => {
        const target = nodeMap.get(syn.targetId);
        return target && target.columnId !== node.columnId;
    }).length;

    document.getElementById('np-links').innerHTML = `
        Connections: <span style="color: #fff;">${node.outgoing.length}</span><br>
        Inter-Column (Voting): <span style="color: #4ade80;">${interColumnLinks}</span>
    `;
}

export function updatePassportStatus(node) {
    let status = [];
    if (node.statusFlags & FLAG_IS_PREDICTIVE) status.push("Predictive");
    if (node.statusFlags & FLAG_IS_INHIBITED) status.push("Inhibited (WTA)");
    if (node.vState > 0.8) status.push("Active (Spike)");
    
    if (status.length === 0) status.push("Idle");
    document.getElementById('np-status').innerText = status.join(" | ");
}

export function updateClassifierPanel(predictions) {
    if (!predictions || predictions.length < 10) return;
    
    // Check if label is provided as 11th element (if not, it's undefined)
    const currentLabel = predictions.length > 10 ? predictions[10] : -1;

    const panel = document.getElementById('classifier-panel');
    if (panel) panel.style.display = 'block';

    // 1. Exponential Moving Average Smoothing
    for (let i = 0; i < 10; i++) {
        smoothedPredictions[i] = (smoothedPredictions[i] * 0.8) + (predictions[i] * 0.2);
    }

    let maxVal = -Infinity;
    let maxIdx = -1;
    let minVal = Infinity;

    for (let i = 0; i < 10; i++) {
        if (smoothedPredictions[i] > maxVal) { maxVal = smoothedPredictions[i]; maxIdx = i; }
        if (smoothedPredictions[i] < minVal) { minVal = smoothedPredictions[i]; }
    }

    // 2. Winner Hysteresis & Hold logic
    if (winnerHoldTicks > 0) {
        setWinnerHoldTicks(winnerHoldTicks - 1);
    } else {
        if (lastDisplayedWinner !== -1) {
            // Require 15% margin over the current winner to switch
            const currentWinnerVal = smoothedPredictions[lastDisplayedWinner];
            const range = Math.max(1.0, maxVal - minVal);
            if (maxVal > currentWinnerVal + (range * 0.15)) {
                setLastDisplayedWinner(maxIdx);
                setWinnerHoldTicks(30);
            }
        } else {
            setLastDisplayedWinner(maxIdx);
            setWinnerHoldTicks(30);
        }
    }

    const range = Math.max(1.0, maxVal - minVal);

    // 3. UI Update
    for (let i = 0; i < 10; i++) {
        const row = document.getElementById(`class-row-${i}`);
        const bar = document.getElementById(`bar-${i}`);
        const valText = document.getElementById(`val-${i}`);
        
        if (row && bar && valText) {
            let pct = ((smoothedPredictions[i] - minVal) / range) * 100;
            pct = Math.max(0, Math.min(100, pct));
            
            bar.style.width = `${pct}%`;
            valText.innerText = `${Math.round(pct)}%`;
            
            // Reset classes
            row.classList.remove('winner', 'bar-label', 'bar-prediction', 'bar-correct');
            
            // Apply new classes
            if (i === currentLabel) {
                row.classList.add('bar-label');
            }
            if (i === lastDisplayedWinner && pct > 20) {
                row.classList.add('bar-prediction');
                if (i === currentLabel) {
                    row.classList.add('bar-correct');
                }
            }
        }
    }

    const guessText = document.getElementById('network-guess');
    if (guessText) {
        if (maxVal > -50.0 && lastDisplayedWinner !== -1) {
            guessText.innerText = `[${lastDisplayedWinner}]`;
            if (lastDisplayedWinner === currentLabel) {
                guessText.style.color = '#3fff00';
                guessText.style.textShadow = '0 0 10px #3fff00';
            } else {
                guessText.style.color = '#00fbff';
                guessText.style.textShadow = '0 0 10px #00fbff';
            }
        } else {
            guessText.innerText = `[-]`;
            guessText.style.color = 'white';
            guessText.style.textShadow = 'none';
        }
    }
}

export function buildLegend() {
    const container = document.getElementById('legend-content');
    if (!container) return;
    container.innerHTML = '';
    const names = {0:"PYRAMIDAL", 1:"BIPOLAR", 2:"INTERNEURON", 3:"PACEMAKER", 4:"PURKINJE", 5:"PHOTORECEPTOR", 6:"NOCICEPTOR", dead_calcium:"DEAD (CALCIUM)", dead_atp:"DEAD (STARVED)", dead_regen:"REGENERATING", dead:"DEAD (OTHER)"};
    
    import('./config.js').then(config => {
        Object.keys(config.TYPE_COLORS).forEach(k => {
            const item = document.createElement('div'); 
            item.className = 'legend-item';
            item.innerHTML = `<div class="color-dot" style="background-color: #${config.TYPE_COLORS[k].getHexString()}"></div>
                              <span class="label">${names[k]}</span>
                              <span id="count-${k}" class="pop-count">[0]</span>`;
            container.appendChild(item);
        });
    });
}

export function updateLegendCounts() {
    const counts = {0:0, 1:0, 2:0, 3:0, 4:0, 5:0, 6:0, dead:0, dead_calcium:0, dead_atp:0, dead_regen:0};
    nodeMap.forEach(n => { 
        if(!n.isAlive) {
            counts[n.deathReason || 'dead']++;
        } else { 
            counts[n.type]++; 
        }
    });
    Object.keys(counts).forEach(t => {
        const el = document.getElementById(`count-${t}`);
        if(el) el.innerText = `[${counts[t]}]`;
    });
}

export function initUIEventListeners() {
    const canvas = document.getElementById('retina-canvas');
    if (canvas) {
        retinaCtx = canvas.getContext('2d');
        retinaCtx.imageSmoothingEnabled = false;
    }

    // Initialize Classifier Rows
    const classifierRowsContainer = document.getElementById('classifier-rows');
    if (classifierRowsContainer && classifierRowsContainer.children.length === 0) {
        for (let i = 0; i < 10; i++) {
            const row = document.createElement('div');
            row.className = 'classifier-row';
            row.id = `class-row-${i}`;
            row.innerHTML = `
                <span class="digit-label">${i}</span>
                <div class="bar-container"><div id="bar-${i}" class="bar"></div></div>
                <span id="val-${i}" class="digit-val">0%</span>
            `;
            classifierRowsContainer.appendChild(row);
        }
    }

    const dSlider = document.getElementById('inp-dopamine');
    if (dSlider) {
        dSlider.addEventListener('input', (e) => {
            const val = parseFloat(e.target.value);
            document.getElementById('v-dopamine').innerText = val.toFixed(1);
            const ws = getWs();
            if (ws) ws.send(JSON.stringify({ type: "UpdateEnvironment", payload: { dopamine_multiplier: val, stdp_learning_rate: currentEnv.lr } }));
        });
    }

    const lrSlider = document.getElementById('inp-lr');
    if (lrSlider) {
        lrSlider.addEventListener('input', (e) => {
            const val = parseFloat(e.target.value);
            document.getElementById('v-lr').innerText = val.toFixed(2);
            currentEnv.lr = val;
            const ws = getWs();
            if (ws) ws.send(JSON.stringify({ type: "UpdateEnvironment", payload: { dopamine_multiplier: currentEnv.dopamine, stdp_learning_rate: val } }));
        });
    }

    // Attach global functions to window for onclick handlers in HTML
    window.toggleLayer = toggleLayer;
    window.toggleColorMode = toggleColorMode;
}

export function saveBrain() {
    const ws = getWs();
    if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ type: "SaveBrain" }));
        console.log("Save brain request sent");
    } else {
        console.warn("WebSocket not connected, cannot save brain.");
    }
}
window.saveBrain = saveBrain;
