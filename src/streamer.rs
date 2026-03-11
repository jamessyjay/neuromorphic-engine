use serde::Serialize;
use std::sync::Arc;
use tokio::sync::broadcast;
use warp::Filter;
use futures_util::{StreamExt, SinkExt}; // needed for ws.split() and sender.send()

#[derive(Serialize)]
pub struct StateSnapshot {
    pub tick: u32,
    pub neurons: Vec<(u32, u16, u16, u8)>, // id, x, y, visual_state
    pub synapses: Vec<(u32, u32, f32)>,   // source, target, weight
}

/// Starts a WebSocket server on port 3030 that broadcasts whatever it receives on `rx`.
pub async fn start_server(tx: broadcast::Sender<String>) {
    // We only need a receiver for each new connection
    let tx = Arc::new(tx);

    let tx_filter = warp::any().map(move || Arc::clone(&tx));

    let stream_route = warp::path("stream")
        .and(warp::ws())
        .and(tx_filter)
        .map(|ws: warp::ws::Ws, tx: Arc<broadcast::Sender<String>>| {
            ws.on_upgrade(move |socket| handle_client(socket, tx))
        });

    let routes = stream_route.with(warp::cors().allow_any_origin());

    println!("WebSocket server started on ws://127.0.0.1:3030/stream");
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

async fn handle_client(ws: warp::ws::WebSocket, tx: Arc<broadcast::Sender<String>>) {
    let mut rx = tx.subscribe();
    let (mut client_ws_sender, _) = ws.split();

    while let Ok(msg) = rx.recv().await {
        if let Err(_e) = client_ws_sender.send(warp::ws::Message::text(msg)).await {
            // Client probably disconnected
            break;
        }
    }
}
