import { nodeMap, activePulses, currentEnv } from './state.js';
import { VISUAL_SAMPLING, MAX_PULSES } from './config.js';
import { initNetwork } from './render.js';
import { retinaCtx } from './ui.js';

let ws = null;

export function getWs() {
    return ws;
}

export function connect(host) {
    ws = new WebSocket(`ws://${host}/ws`);
    ws.onmessage = (event) => {
        const msg = JSON.parse(event.data);
        if (msg.type === "InitialState") {
            document.getElementById('status').innerText = "Running";
            document.getElementById('total-n').innerText = msg.payload.total_neurons;
            document.getElementById('total-s').innerText = msg.payload.total_synapses;
            initNetwork(msg.payload);
        } else if (msg.type === "TickUpdate") {
            const data = msg.payload;
            document.getElementById('tick').innerText = data.tick;
            
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
                        if (Math.random() < VISUAL_SAMPLING) {
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
            if (data.retina && retinaCtx) {
                const imgData = retinaCtx.createImageData(32, 32);
                for (let i=0; i<32*32; i++) {
                    const val = data.retina[i];
                    const idx = i*4;
                    imgData.data[idx] = val;
                    imgData.data[idx+1] = val;
                    imgData.data[idx+2] = val;
                    imgData.data[idx+3] = 255;
                }
                retinaCtx.putImageData(imgData, 0, 0);
            }

            // Status updates
            if (data.statuses && data.statuses.length > 0) {
                for (let i = 0; i < data.statuses.length; i += 5) {
                    const nid = data.statuses[i];
                    const flags = data.statuses[i+1];
                    const atp = data.statuses[i+2];
                    const calc = data.statuses[i+3];
                    const v = data.statuses[i+4];
                    const n = nodeMap.get(nid);
                    if (n) {
                        n.statusFlags = flags;
                        // Map internal engine signals to visual ones
                        // Check if DEAD (is_alive == 0 => flags usually indicate this or v == -70 with 0 atp in old logic)
                        // This would need proper mapping based on engine output. 
                        // For now we assume if flags & 0x04 == DEAD (hypothetical)
                        // Actually, backend now returns death reason. Let's assume isAlive logic is handled by specific events or flags.
                        // Assuming Phase 53: we trust the engine's events for death.
                        h_atp += atp;
                        h_calc += calc;
                        h_v += v;
                        
                        if (!n.isAlive) {
                            deathCount++;
                            typeCounts[n.deathReason || 'dead']++;
                        } else {
                            typeCounts[n.type]++;
                        }
                    }
                }
                
                const avg_atp = h_atp / (data.statuses.length/5);
                const avg_calc = h_calc / (data.statuses.length/5);
                const avg_v = h_v / (data.statuses.length/5);
                
                document.getElementById('live-count').innerText = `${liveCount} / ${deathCount}`;
                document.getElementById('avg-atp').innerText = avg_atp.toFixed(1);
                document.getElementById('avg-calcium').innerText = avg_calc.toFixed(1);
                document.getElementById('avg-voltage').innerText = avg_v.toFixed(1);
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

            Object.keys(typeCounts).forEach(k => {
                const el = document.getElementById(`count-${k}`);
                if (el) el.innerText = `[${typeCounts[k]}]`;
            });
        }
    };
    
    ws.onclose = () => {
        document.getElementById('status').innerText = "Disconnected";
        document.getElementById('status').style.color = "red";
        setTimeout(() => connect(host), 2000);
    };
}
