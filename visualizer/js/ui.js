import { layerVisibility, colorMode, setColorMode, nodeMap, currentEnv, TYPE_COLORS, FLAG_IS_PREDICTIVE, FLAG_IS_INHIBITED } from './state.js';
import { getWs } from './network.js';

export let retinaCtx = null;

window.addEventListener('load', () => {
    const canvas = document.getElementById('retina-canvas');
    if (canvas) retinaCtx = canvas.getContext('2d');
});

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

export function buildLegend() {
    const container = document.getElementById('legend-content');
    if (!container) return;
    container.innerHTML = '';
    const names = {0:"PYRAMIDAL", 1:"BIPOLAR", 2:"INTERNEURON", 3:"PACEMAKER", 4:"PURKINJE", 5:"PHOTORECEPTOR", 6:"NOCICEPTOR", dead_calcium:"DEAD (CALCIUM)", dead_atp:"DEAD (STARVED)", dead_regen:"REGENERATING", dead:"DEAD (OTHER)"};
    
    import('./config.js').then(config => {
        Object.keys(config.TYPE_COLORS).forEach(k => {
            const item = document.createElement('div'); item.className = 'legend-item';
            item.innerHTML = `<div class="color-dot" style="background-color: #${config.TYPE_COLORS[k].getHexString()}"></div>
                              <span class="label">${names[k]}</span>
                              <span id="count-${k}" class="pop-count">[0]</span>`;
            container.appendChild(item);
        });
    });
}

export function initUIEventListeners() {
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
