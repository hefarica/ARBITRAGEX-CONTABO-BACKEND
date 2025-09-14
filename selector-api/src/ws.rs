use axum::extract::ws::{Message, WebSocket};
use futures::{sink::SinkExt, stream::StreamExt};
use redis::AsyncCommands;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{error, info};

use crate::models::{WebSocketMessage, WsEventType};
use crate::state::AppState;

pub async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    
    info!("New WebSocket connection established");

    // Spawn heartbeat task
    let heartbeat_state = state.clone();
    let mut heartbeat_sender = sender.clone();
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(heartbeat_state.config.ws_heartbeat_interval));
        loop {
            interval.tick().await;
            let msg = WebSocketMessage {
                event_type: WsEventType::Heartbeat,
                data: serde_json::json!({"ping": "pong"}),
                timestamp: chrono::Utc::now(),
            };
            
            if let Ok(json) = serde_json::to_string(&msg) {
                if heartbeat_sender.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });

    // Spawn Redis subscription task
    let redis_state = state.clone();
    let mut redis_sender = sender.clone();
    tokio::spawn(async move {
        if let Err(e) = subscribe_to_opportunities(redis_state, &mut redis_sender).await {
            error!("Redis subscription error: {}", e);
        }
    });

    // Handle incoming messages
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Text(text) => {
                info!("Received message: {}", text);
                // Handle client messages if needed
            }
            Message::Close(_) => {
                info!("WebSocket connection closed");
                break;
            }
            _ => {}
        }
    }
}

async fn subscribe_to_opportunities(
    state: Arc<AppState>,
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
) -> anyhow::Result<()> {
    let redis_client = redis::Client::open(state.config.redis_url.as_str())?;
    let mut pubsub = redis_client.get_async_connection().await?.into_pubsub();
    
    // Subscribe to opportunity updates
    pubsub.subscribe("opportunities").await?;
    pubsub.subscribe("executions").await?;
    
    let mut stream = pubsub.on_message();
    
    while let Some(msg) = stream.next().await {
        let channel = msg.get_channel_name();
        let payload: String = msg.get_payload()?;
        
        let event_type = match channel {
            "opportunities" => WsEventType::OpportunityNew,
            "executions" => WsEventType::ExecutionStart,
            _ => continue,
        };
        
        let ws_msg = WebSocketMessage {
            event_type,
            data: serde_json::from_str(&payload)?,
            timestamp: chrono::Utc::now(),
        };
        
        if let Ok(json) = serde_json::to_string(&ws_msg) {
            if sender.send(Message::Text(json)).await.is_err() {
                break;
            }
        }
    }
    
    Ok(())
}



