use serde::Serialize;
use std::sync::Arc;
use tokio::sync::broadcast;
use warp::Filter;
use futures_util::{StreamExt, SinkExt};

#[derive(Serialize, Clone)]
pub struct StateSnapshot {
    pub tick: u32,
    pub total_neurons: usize,
    pub total_synapses: usize,
    pub neurons: Vec<(u32, f32, f32, f32, f32, f32, f32, u8)>, // id, x, y, z, voltage, threshold, atp_pct, type
    pub synapses: Vec<(u32, u32, f32, f32)>,                // source_id, target_id, weight, buffer
}

pub async fn start_server(tx: broadcast::Sender<String>) {
    let tx = Arc::new(tx);
    let tx_filter = warp::any().map(move || Arc::clone(&tx));

    let stream_route = warp::path("stream")
        .and(warp::ws())
        .and(tx_filter)
        .map(|ws: warp::ws::Ws, tx: Arc<broadcast::Sender<String>>| {
            ws.on_upgrade(move |socket| handle_client(socket, tx))
        });

    // Also serve the visualizer directory for convenience
    let static_files = warp::fs::dir("visualizer");
    let routes = stream_route.or(static_files).with(warp::cors().allow_any_origin());

    println!("WebSocket server started on ws://127.0.0.1:3030/stream");
    println!("Visualizer available at http://127.0.0.1:3030/index.html");
    
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

async fn handle_client(ws: warp::ws::WebSocket, tx: Arc<broadcast::Sender<String>>) {
    let mut rx = tx.subscribe();
    let (mut client_ws_sender, _) = ws.split();

    println!("New WebSocket client connected!");

    loop {
        match rx.recv().await {
            Ok(msg) => {
                if let Err(e) = client_ws_sender.send(warp::ws::Message::text(msg)).await {
                    println!("WebSocket send error: {}. Closing connection.", e);
                    break;
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                println!("WebSocket client lagged, skipped {} messages. Recovering...", skipped);
                // The loop will just continue and try to recv the next fresh message
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                println!("Broadcast channel closed.");
                break;
            }
        }
    }
}
