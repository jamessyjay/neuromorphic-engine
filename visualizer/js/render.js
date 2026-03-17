import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { 
    nodeMap, activePulses, neuronMeshes, synapseLines, pulseSystem, 
    colorMode, selectedNeuronId, layerVisibility,
    setNeuronMeshes, setSynapseLines, setPulseSystem, setSelectedNeuronId 
} from './state.js';
import { TYPE_COLORS, MAX_PULSES, PULSE_SPEED, FLAG_IS_INHIBITED, FLAG_IS_PREDICTIVE } from './config.js';
import { showPassport, updatePassportStatus } from './ui.js';

let scene, camera, renderer, controls;
const raycaster = new THREE.Raycaster();
const mouse = new THREE.Vector2();

const _tempColor = new THREE.Color();
const _dummy = new THREE.Object3D();

export function initRenderer() {
    scene = new THREE.Scene();
    camera = new THREE.PerspectiveCamera(60, window.innerWidth / window.innerHeight, 1, 10000);
    renderer = new THREE.WebGLRenderer({ antialias: true, powerPreference: "high-performance" });
    renderer.setSize(window.innerWidth, window.innerHeight);
    document.body.appendChild(renderer.domElement);

    controls = new OrbitControls(camera, renderer.domElement);
    camera.position.set(0, 0, 200);

    window.addEventListener('resize', () => { 
        camera.aspect = window.innerWidth / window.innerHeight; 
        camera.updateProjectionMatrix(); 
        renderer.setSize(window.innerWidth, window.innerHeight); 
    });

    window.addEventListener('click', (event) => {
        mouse.x = (event.clientX / window.innerWidth) * 2 - 1;
        mouse.y = -(event.clientY / window.innerHeight) * 2 + 1;
        raycaster.setFromCamera(mouse, camera);

        let closest = null;
        let closestDist = Infinity;

        for (const mData of neuronMeshes) {
            const intersects = raycaster.intersectObject(mData.mesh);
            if (intersects.length > 0) {
                if (intersects[0].distance < closestDist) {
                    closestDist = intersects[0].distance;
                    closest = { mesh: mData.mesh, instanceId: intersects[0].instanceId };
                }
            }
        }

        if (closest) {
            for (const [id, node] of nodeMap.entries()) {
                if (node.mesh === closest.mesh && node.instanceId === closest.instanceId) {
                    setSelectedNeuronId(id);
                    showPassport(node);
                    break;
                }
            }
        } else {
            setSelectedNeuronId(null);
            document.getElementById('neuron-passport').style.display = 'none';
        }
    });

    requestAnimationFrame(animate);
}

function createGlowTexture() {
    const canvas = document.createElement('canvas');
    canvas.width = 64; canvas.height = 64;
    const ctx = canvas.getContext('2d');
    const gradient = ctx.createRadialGradient(32, 32, 0, 32, 32, 32);
    gradient.addColorStop(0, 'white'); gradient.addColorStop(1, 'transparent');
    ctx.fillStyle = gradient; ctx.fillRect(0, 0, 64, 64);
    return new THREE.CanvasTexture(canvas);
}
const glowTexture = createGlowTexture();

export function initNetwork(data) {
    neuronMeshes.forEach(m => scene.remove(m.mesh));
    setNeuronMeshes([]);
    
    if (synapseLines) {
        scene.remove(synapseLines);
        if (synapseLines.geometry) synapseLines.geometry.dispose();
        if (synapseLines.material) synapseLines.material.dispose();
        setSynapseLines(null);
    }
    if (pulseSystem) {
        scene.remove(pulseSystem);
        setPulseSystem(null);
    }
    nodeMap.clear();
    activePulses.length = 0;

    if (!data.neurons) return;

    const counts = {};
    data.neurons.forEach(n => counts[n[4]] = (counts[n[4]] || 0) + 1);

    const geometries = { 5: new THREE.TetrahedronGeometry(2), 4: new THREE.OctahedronGeometry(2), 3: new THREE.BoxGeometry(3,3,3), default: new THREE.SphereGeometry(1.5,8,8) };
    
    const newMeshes = [];
    Object.keys(counts).forEach(t => {
        const mesh = new THREE.InstancedMesh(geometries[t] || geometries.default, new THREE.MeshBasicMaterial({color: 0xffffff}), counts[t]);
        scene.add(mesh);
        newMeshes.push({ type: parseInt(t), mesh, currentIndex: 0 });
    });
    setNeuronMeshes(newMeshes);

    data.neurons.forEach(n => {
        const id = n[0], type = n[4];
        const columnId = n[5];
        const layerId = n[6];
        const minicolumnId = n[7];
        const statusFlags = n[8] || 0;

        let x = (n[1] / 65535) * 600 - 300;
        let y = (n[2] / 65535) * 600 - 300;
        let z = (n[3] / 65535) * 400 - 200;

        const mData = neuronMeshes.find(m => m.type === type);
        
        if (mData) {
            _dummy.position.set(x, y, z);
            _dummy.updateMatrix();
            mData.mesh.setMatrixAt(mData.currentIndex, _dummy.matrix);

            let initialColor = TYPE_COLORS[type] || new THREE.Color(0xffffff);
            mData.mesh.setColorAt(mData.currentIndex, initialColor);

            nodeMap.set(id, { 
                id, x, y, z, type, 
                columnId, layerId, minicolumnId,
                meshIdx: mData.currentIndex,
                instanceId: mData.currentIndex,
                mesh: mData.mesh,
                outgoing: [], 
                visualActivity: 0, 
                isAlive: true,
                statusFlags: statusFlags,
                vState: 0
            });
            mData.currentIndex++;
        }
    });
    neuronMeshes.forEach(m => m.mesh.instanceMatrix.needsUpdate = true);

    if (data.synapses) {
        const pos = new Float32Array(data.synapses.length * 6);
        const col = new Float32Array(data.synapses.length * 6);
        data.synapses.forEach((syn, i) => {
            const s = nodeMap.get(syn[0]), t = nodeMap.get(syn[1]);
            if (s && t) {
                const idx = i*6;
                pos[idx]=s.x; pos[idx+1]=s.y; pos[idx+2]=s.z; pos[idx+3]=t.x; pos[idx+4]=t.y; pos[idx+5]=t.z;
                
                const weight = syn[2] || 1.0;
                let c = new THREE.Color(0x333333);
                
                if (weight < 0.0) {
                    c = new THREE.Color(0x220000); 
                } else if (weight > 1.5) {
                    c = new THREE.Color(0x55ff55);
                } else if (weight < 0.5) {
                    c = new THREE.Color(0x222255);
                }
                
                col[idx]=col[idx+3]=c.r; col[idx+1]=col[idx+4]=c.g; col[idx+2]=col[idx+5]=c.b;
                s.outgoing.push({ targetId: t.id, weight: weight });
            }
        });

        const sGeo = new THREE.BufferGeometry();
        sGeo.setAttribute('position', new THREE.BufferAttribute(pos, 3));
        sGeo.setAttribute('color', new THREE.BufferAttribute(col, 3));
        const sMat = new THREE.LineBasicMaterial({ vertexColors: true, transparent: true, opacity: 0.15, blending: THREE.AdditiveBlending });
        const newSynapseLines = new THREE.LineSegments(sGeo, sMat);
        scene.add(newSynapseLines);
        setSynapseLines(newSynapseLines);
    }

    const pulseGeometry = new THREE.BufferGeometry();
    const pulsePositions = new Float32Array(MAX_PULSES * 3);
    pulseGeometry.setAttribute('position', new THREE.BufferAttribute(pulsePositions, 3));
    pulseGeometry.attributes.position.setUsage(THREE.DynamicDrawUsage);
    
    const pMat = new THREE.PointsMaterial({ size: 1.5, map: glowTexture, color: 0xffffff, transparent: true, opacity: 1.0, depthWrite: false, blending: THREE.AdditiveBlending });
    const newPulseSystem = new THREE.Points(pulseGeometry, pMat);
    scene.add(newPulseSystem);
    setPulseSystem(newPulseSystem);
}

function animate() {
    requestAnimationFrame(animate);
    
    let activeWtaCount = 0;

    neuronMeshes.forEach(m => {
        let matrixUpdated = false;
        
        nodeMap.forEach((n, id) => {
            if (n.type === m.type) {
                
                if (!layerVisibility[n.layerId]) {
                    if (n.isVisible !== false) {
                        _dummy.position.set(n.x, n.y, n.z);
                        _dummy.scale.set(0, 0, 0); // Hide
                        _dummy.updateMatrix();
                        m.mesh.setMatrixAt(n.meshIdx, _dummy.matrix);
                        n.isVisible = false;
                        matrixUpdated = true;
                    }
                    return; // Skip drawing when hidden
                } else {
                    if (n.isVisible === false) {
                        _dummy.position.set(n.x, n.y, n.z);
                        _dummy.scale.set(1, 1, 1);
                        _dummy.updateMatrix();
                        m.mesh.setMatrixAt(n.meshIdx, _dummy.matrix);
                        n.isVisible = true;
                        matrixUpdated = true;
                    }
                }

                n.visualActivity *= 0.94;

                let baseColor;
                if (colorMode === 'layer') {
                    switch(n.layerId) {
                        case 6: baseColor = new THREE.Color(0x000080); break;
                        case 5: baseColor = new THREE.Color(0xff0000); break;
                        case 4: baseColor = new THREE.Color(0xffff00); break;
                        case 2: baseColor = new THREE.Color(0x00ffff); break;
                        case 1: baseColor = new THREE.Color(0xffffff); break;
                        default: baseColor = new THREE.Color(0x444444);
                    }
                } else {
                    baseColor = TYPE_COLORS[n.type] || new THREE.Color(0x444444);
                }

                if (n.statusFlags & FLAG_IS_INHIBITED) {
                    activeWtaCount++;
                }

                if (!n.isAlive) {
                    if (n.deathReason === 'calcium') _tempColor.copy(TYPE_COLORS.dead_calcium);
                    else if (n.deathReason === 'atp') _tempColor.copy(TYPE_COLORS.dead_atp);
                    else if (n.deathReason === 'regen') _tempColor.copy(TYPE_COLORS.dead_regen);
                    else _tempColor.copy(TYPE_COLORS.dead);
                } else {
                    if (colorMode === 'layer') {
                        // В режиме LAYER игнорим предиктив стейт и ингибирование - показываем чистые цвета слоев + вспышки спайков
                        _tempColor.copy(baseColor).lerp(new THREE.Color(1,1,1), n.visualActivity);
                    } else {
                        if (n.statusFlags & FLAG_IS_INHIBITED) {
                            _tempColor.copy(baseColor).multiplyScalar(0.1);
                        } else if (n.statusFlags & FLAG_IS_PREDICTIVE) {
                            const predColor = new THREE.Color(0xff00ff);
                            _tempColor.copy(predColor).lerp(new THREE.Color(1,1,1), n.visualActivity);
                        } else {
                            _tempColor.copy(baseColor).lerp(new THREE.Color(1,1,1), n.visualActivity);
                        }
                    }
                }
                
                m.mesh.setColorAt(n.meshIdx, _tempColor);
                if (selectedNeuronId === n.id) {
                    updatePassportStatus(n);
                }
            }
        });
        m.mesh.instanceColor.needsUpdate = true;
        if (matrixUpdated) {
            m.mesh.instanceMatrix.needsUpdate = true;
        }
    });

    const wtaElement = document.getElementById('wta-active');
    if (wtaElement) wtaElement.innerText = activeWtaCount;

    if (pulseSystem && activePulses.length > 0) {
        const pos = pulseSystem.geometry.attributes.position.array;
        for (let i = 0; i < MAX_PULSES; i++) {
            const idx = i*3;
            if (i < activePulses.length) {
                const p = activePulses[i]; p.p += PULSE_SPEED;
                if (p.p >= 1.0) { activePulses.splice(i, 1); i--; continue; }
                pos[idx]=p.s.x+(p.t.x-p.s.x)*p.p; pos[idx+1]=p.s.y+(p.t.y-p.s.y)*p.p; pos[idx+2]=p.s.z+(p.t.z-p.s.z)*p.p;
            } else { pos[idx]=pos[idx+1]=pos[idx+2]=10000; }
        }
        pulseSystem.geometry.attributes.position.needsUpdate = true;
    }
    controls.update();
    renderer.render(scene, camera);
}
