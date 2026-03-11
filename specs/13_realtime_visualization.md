# Spec 13: Real-Time Visualization & Decoupled Observer

## 1. Concept

The Neuromorphic Engine operates at ultra-high speeds (~9000 TPS). Synchronous UI rendering would destroy this performance. To observe the network in real-time, we implement the **Decoupled Observer Pattern**.
The Engine (Core) acts as a broadcaster. The UI acts as a passive, independent listener.

## 2. Architecture

- **Backend (Rust):** A non-blocking, async WebSocket server running on a separate thread (via `tokio`).
- **Communication:** The Core math loop samples the network state every `N` ticks (e.g., every 300 ticks to yield ~30 FPS at 9000 TPS) and uses a non-blocking `try_send` over a channel. If the UI is slow to read, the Core simply drops the frame and continues calculating. **The math loop must NEVER wait for the network.**
- **Frontend (HTML/JS):** A standalone, zero-dependency `index.html` file using the HTML5 `<canvas>` API to render the network graph.

## 3. Data Payload (The Snapshot)

To keep the WebSocket payload tiny, we only send what is necessary for drawing:

- `tick`: Current simulation tick.
- `neurons`: Array of `[id, x, y, z, state]`.
  - `state`: 0 = Sleeping/Resting, 1 = Spiking (NMDA or Soma), 2 = Low ATP (Warning).
- `synapses`: Array of `[source_id, target_id, weight_normalized]`.

## 4. Visual Language

- **Background:** Deep Black (Void).
- **Neurons:**
  - Resting: Dim Blue / Cyan.
  - Spiking: Brilliant Yellow / White flash.
  - Low Energy: Red outline.
- **Synapses:**
  - Rendered as lines connecting neurons.
  - Opacity/Thickness is mapped directly to synaptic `weight`.
  - Pruned synapses disappear automatically.
