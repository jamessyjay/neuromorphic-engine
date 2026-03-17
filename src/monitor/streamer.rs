use serde::{Serialize, Deserialize};
use std::sync::{Arc, Mutex};
use tokio::sync::{broadcast, RwLock};
use warp::Filter;
use futures_util::{StreamExt, SinkExt};


#[derive(Deserialize, Debug)]
#[serde(tag = "type", content = "payload")]
pub enum IncomingMsg {
    UpdateEnvironment {
        dopamine_multiplier: f32,
        stdp_learning_rate: f32,
    },
    SaveBrain,
}

#[derive(Serialize, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum StreamMessage {
    InitialState(InitialStatePayload),
    TickUpdate(TickUpdatePayload),
}

#[derive(Serialize, Clone)]
pub struct InitialStatePayload {
    pub total_neurons: usize,
    pub total_synapses: usize,
    pub neurons: Vec<(u32, f32, f32, f32, u8, u16, u8, u8, u32)>, // id, x, y, z, type, column_id, layer_id, minicolumn_id
    pub synapses: Vec<(u32, u32, f32)>,         // source_id, target_id, weight
}

#[derive(Serialize, Clone)]
pub struct TickUpdatePayload {
    pub tick: u32,
    pub total_spikes: u32,
    pub global_oxygen: f32,
    pub global_glucose: f32,
    pub global_dopamine: f32,
    pub neurons: Vec<(u32, f32, f32, u32)>, // id, visual_state, atp_pct, status_flags
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synapses: Option<Vec<(u32, u32, f32)>>, // source_id, target_id, weight
    pub synapse_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retina_frame: Option<Vec<f32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub predictions: Option<Vec<f32>>,
}

pub async fn start_server(tx: broadcast::Sender<String>, shared_env: Arc<Mutex<crate::engine::ffi::Environment>>) {
    let tx = Arc::new(tx);
    let tx_for_filter = tx.clone();
    let tx_for_bg = tx.clone();
    let tx_filter = warp::any().map(move || Arc::clone(&tx_for_filter));
    
    let initial_state_cache: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
    let cache_clone = initial_state_cache.clone();
    
    // Background task to cache InitialState for late joiners
    let mut rx = tx_for_bg.subscribe();
    tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if msg.contains("\"type\":\"InitialState\"") {
                let mut cache = cache_clone.write().await;
                *cache = Some(msg);
                println!("Cached InitialState for late UI joiners.");
            }
        }
    });

    let cache_filter = warp::any().map(move || Arc::clone(&initial_state_cache));
    let env_filter = warp::any().map(move || Arc::clone(&shared_env));

    let stream_route = warp::path("stream")
        .and(warp::ws())
        .and(tx_filter)
        .and(cache_filter)
        .and(env_filter)
        .map(|ws: warp::ws::Ws, tx: Arc<broadcast::Sender<String>>, cache: Arc<RwLock<Option<String>>>, shared_env: Arc<Mutex<crate::engine::ffi::Environment>>| {
            ws.on_upgrade(move |socket| handle_client(socket, tx, cache, shared_env))
        });

    let static_files = warp::fs::dir("visualizer");
    let routes = stream_route.or(static_files).with(warp::cors().allow_any_origin());

    println!("WebSocket server started on ws://127.0.0.1:3030/stream");
    println!("Visualizer available at http://127.0.0.1:3030/index.html");
    
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

async fn handle_client(ws: warp::ws::WebSocket, tx: Arc<broadcast::Sender<String>>, cache: Arc<RwLock<Option<String>>>, shared_env: Arc<Mutex<crate::engine::ffi::Environment>>) {
    let mut rx = tx.subscribe();
    let (mut client_ws_sender, mut client_ws_rcv) = ws.split();

    
    // Spawn task to receive messages from client
    let env_for_rcv = shared_env.clone();
    tokio::spawn(async move {
        while let Some(result) = client_ws_rcv.next().await {
            match result {
                Ok(msg) => {
                    if let Ok(text) = msg.to_str() {
                        if let Ok(incoming) = serde_json::from_str::<IncomingMsg>(text) {
                            match incoming {
                                IncomingMsg::UpdateEnvironment { dopamine_multiplier, stdp_learning_rate } => {
                                    if let Ok(mut env) = env_for_rcv.try_lock() {
                                        env.dopamine_multiplier = dopamine_multiplier;
                                        env.stdp_learning_rate = stdp_learning_rate;
                                        println!("Chemistry updated: Dopamine x{}, LR: {}", dopamine_multiplier, stdp_learning_rate);
                                    }
                                }
                                IncomingMsg::SaveBrain => {
                                    println!("SAVE BRAIN trigger received from UI.");
                                    let _ = std::fs::write("SAVE_BRAIN_NOW", "1");
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    // silently break on WS error (browser refresh)
                    break;
                }
            }
        }
    });

    println!("New WebSocket client connected!");

    // Send cached initial state immediately to new clients
    if let Some(init_msg) = cache.read().await.as_ref() {
        if let Err(e) = client_ws_sender.send(warp::ws::Message::text(init_msg.clone())).await {
            println!("Error sending initial state: {}", e);
            return;
        }
        println!("Sent cached InitialState to new client.");
    }

    loop {
        match rx.recv().await {
            Ok(msg) => {
                if let Err(_) = client_ws_sender.send(warp::ws::Message::text(msg)).await {
                    // WebSocket send error - quiet close on refresh
                    break;
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                println!("WebSocket client lagged, skipped {} messages. Recovering...", skipped);
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                println!("Broadcast channel closed.");
                break;
            }
        }
    }
}
