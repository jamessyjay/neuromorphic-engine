import { nodeMap, activePulses, currentEnv } from './state.js';
import { MAX_PULSES } from './config.js';
import { initNetwork } from './render.js';
import { retinaCtx, updateLegendCounts, updateClassifierPanel } from './ui.js';



let ws = null;

export function getWs() {
    return ws;
}

export function connect(host) {
    ws = new WebSocket(`ws://${host}/stream`);
    ws.onmessage = (event) => {
        const msg = JSON.parse(event.data);
        if (msg.type === "InitialState") {
            document.getElementById('status').innerText = "STATUS: ACTIVE";
            document.getElementById('status').style.color = '#4ade80';
            document.getElementById('neurons').innerText = msg.payload.total_neurons;
            document.getElementById('synapses').innerText = msg.payload.total_synapses;
            initNetwork(msg.payload);
        } else if (msg.type === "TickUpdate") {
            const data = msg.payload;
            document.getElementById('tick').innerText = "Tick: " + data.tick;
            
            // Log panel
            if (data.events && data.events.length > 0) {
                const log = document.getElementById('log-container');
                data.events.forEach(ev => {
                    const el = document.createElement('div');
                    el.innerText = `[T:${data.tick}] ${ev}`;
                    el.style.color = ev.includes("DEAD") ? "#ff4444" : "#86efac";
                    log.appendChild(el);
                });
                log.scrollTop = log.scrollHeight;
            }

            // Global stats
            let deathCount = 0;
            let h_atp = 0.0, h_calc = 0.0, h_v = 0.0;
            const liveCount = nodeMap.size - deathCount;

            const typeCounts = {0:0,1:0,2:0,3:0,4:0,5:0,6:0, dead_calcium:0, dead_atp:0, dead_regen:0, dead:0};

            // Spikes (Phase 13 optimizations)
            if (data.spikes && data.spikes.length > 0) {
                data.spikes.forEach(sid => {
                    const n = nodeMap.get(sid);
                    if (n) {
                        n.visualActivity = 1.0;
                        n.vState = 1.0;
                        
                        // Fire visual pulses
                        if (Math.random() < 1.0) {
                            n.outgoing.forEach(out => {
                                if (activePulses.length < MAX_PULSES) {
                                    const targetNode = nodeMap.get(out.targetId);
                                    if (targetNode) {
                                        activePulses.push({ s: n, t: targetNode, p: 0 });
                                    }
                                }
                            });
                        }
                    }
                });
            }
            // Retina Processing
            if (data.retina_frame && retinaCtx) {
                const frame = data.retina_frame;
                const imgData = retinaCtx.createImageData(28, 28);
                for (let i = 0; i < 784; i++) {
                    const val = Math.floor(frame[i] * 255);
                    const idx = i * 4;
                    imgData.data[idx] = val;
                    imgData.data[idx+1] = val;
                    imgData.data[idx+2] = val;
                    imgData.data[idx+3] = 255;
                }
                retinaCtx.putImageData(imgData, 0, 0);
            }

            // Classifier Panel
            if (data.predictions) {
                updateClassifierPanel(data.predictions);
            }


            // Status updates
            if (data.global_oxygen !== undefined) {
                document.getElementById('v-oxygen').innerText = data.global_oxygen.toFixed(2);
                document.getElementById('prog-oxygen').value = data.global_oxygen;
            }
            if (data.global_glucose !== undefined) {
                document.getElementById('v-glucose').innerText = data.global_glucose.toFixed(2);
                document.getElementById('prog-glucose').value = data.global_glucose;
            }
            if (data.global_dopamine !== undefined) {
                document.getElementById('v-dopamine-val').innerText = data.global_dopamine.toFixed(2);
                // Convert -1.0 to 2.0 range into 0 to 1 progress bar for visual
                document.getElementById('prog-dopamine').value = Math.max(0, Math.min(1, (data.global_dopamine + 1.0) / 3.0));
            }
            if (data.synapse_count !== undefined) {
                document.getElementById('synapses').innerText = data.synapse_count;
            }

            let spikesToday = 0, deadToday = 0, activeCount = 0;
            if (data.neurons && data.neurons.length > 0) {
                for (let i = 0; i < data.neurons.length; i++) {
                    const nDataObj = data.neurons[i];
                    const id = nDataObj.id, state = nDataObj.v, atp = nDataObj.a, flags = nDataObj.f || 0;
                    const nData = nodeMap.get(id);
                    if (nData) {
                        if (state >= 3.0 && nData.isAlive) {
                            nData.isAlive = false;
                            deadToday++;
                            if (state === 3.0) nData.deathReason = 'calcium';
                            else if (state === 4.0) nData.deathReason = 'atp';
                            else nData.deathReason = 'regen';
                        } else {
                            nData.isAlive = true;
                            // Trigger visual pulses if moving from un-spiked to spiked state
                            if (state === 1.0 && nData.vState !== 1.0) {
                                if (activePulses.length < MAX_PULSES) {
                                    nData.outgoing.forEach(syn => {
                                        if (Math.random() < 1.0 && activePulses.length < MAX_PULSES) {
                                            const t = nodeMap.get(syn.targetId);
                                            if (t) activePulses.push({ s: nData, t, p: 0 });
                                        }
                                    });
                                }
                            }
                            nData.visualActivity = (state === 1.0) ? 1.0 : nData.visualActivity;
                        }
                        nData.atp = atp;
                        nData.vState = state; 
                        nData.statusFlags = flags;
                    }
                }
                
                document.getElementById('dead-count').innerText = Array.from(nodeMap.values()).filter(n => !n.isAlive).length;
                document.getElementById('spike-rate').innerText = data.total_spikes !== undefined ? data.total_spikes : spikesToday;
            }

            // Sync Environment back from server
            if (data.env) {
                currentEnv.dopamine = data.env.dopamine;
                currentEnv.lr = data.env.lr;
                document.getElementById('inp-dopamine').value = currentEnv.dopamine;
                document.getElementById('v-dopamine').innerText = currentEnv.dopamine.toFixed(1);
                document.getElementById('inp-lr').value = currentEnv.lr;
                document.getElementById('v-lr').innerText = currentEnv.lr.toFixed(2);
            }

            updateLegendCounts();
        }
    };
    
    ws.onclose = () => {
        document.getElementById('status').innerText = "Disconnected";
        document.getElementById('status').style.color = "red";
        setTimeout(() => connect(host), 2000);
    };
}
