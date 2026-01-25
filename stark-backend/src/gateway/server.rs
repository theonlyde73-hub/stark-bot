use crate::channels::ChannelManager;
use crate::db::Database;
use crate::gateway::events::EventBroadcaster;
use crate::gateway::methods;
use crate::gateway::protocol::{ChannelIdParams, RpcError, RpcRequest, RpcResponse};
use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_tungstenite::{accept_async, tungstenite::Message};

pub struct GatewayServer {
    db: Arc<Database>,
    channel_manager: Arc<ChannelManager>,
    broadcaster: Arc<EventBroadcaster>,
}

impl GatewayServer {
    pub fn new(
        db: Arc<Database>,
        channel_manager: Arc<ChannelManager>,
        broadcaster: Arc<EventBroadcaster>,
    ) -> Self {
        Self {
            db,
            channel_manager,
            broadcaster,
        }
    }

    pub async fn run(&self, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let listener = TcpListener::bind(addr).await?;
        log::info!("Gateway WebSocket server listening on {}", addr);

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    log::info!("New WebSocket connection from {}", peer_addr);
                    let db = self.db.clone();
                    let channel_manager = self.channel_manager.clone();
                    let broadcaster = self.broadcaster.clone();

                    tokio::spawn(async move {
                        if let Err(e) =
                            handle_connection(stream, db, channel_manager, broadcaster).await
                        {
                            log::error!("Connection error from {}: {}", peer_addr, e);
                        }
                    });
                }
                Err(e) => {
                    log::error!("Failed to accept connection: {}", e);
                }
            }
        }
    }
}

async fn handle_connection(
    stream: TcpStream,
    db: Arc<Database>,
    channel_manager: Arc<ChannelManager>,
    broadcaster: Arc<EventBroadcaster>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ws_stream = accept_async(stream).await?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Subscribe to events
    let (client_id, mut event_rx) = broadcaster.subscribe();

    // Create a channel for sending messages to the WebSocket
    let (tx, mut rx) = mpsc::channel::<Message>(100);

    // Task to forward messages to WebSocket
    let send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                // Forward RPC responses
                Some(msg) = rx.recv() => {
                    if ws_sender.send(msg).await.is_err() {
                        break;
                    }
                }
                // Forward events
                Some(event) = event_rx.recv() => {
                    if let Ok(json) = serde_json::to_string(&event) {
                        if ws_sender.send(Message::Text(json)).await.is_err() {
                            break;
                        }
                    }
                }
                else => break,
            }
        }
    });

    // Process incoming messages
    while let Some(msg_result) = ws_receiver.next().await {
        match msg_result {
            Ok(Message::Text(text)) => {
                let response = process_request(&text, &db, &channel_manager, &broadcaster).await;
                if let Ok(json) = serde_json::to_string(&response) {
                    let _ = tx.send(Message::Text(json)).await;
                }
            }
            Ok(Message::Ping(data)) => {
                let _ = tx.send(Message::Pong(data)).await;
            }
            Ok(Message::Close(_)) => {
                break;
            }
            Err(e) => {
                log::error!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    // Cleanup
    broadcaster.unsubscribe(&client_id);
    send_task.abort();

    Ok(())
}

async fn process_request(
    text: &str,
    db: &Arc<Database>,
    channel_manager: &Arc<ChannelManager>,
    broadcaster: &Arc<EventBroadcaster>,
) -> RpcResponse {
    // Parse the request
    let request: RpcRequest = match serde_json::from_str(text) {
        Ok(req) => req,
        Err(_) => {
            return RpcResponse::error("".to_string(), RpcError::parse_error());
        }
    };

    let id = request.id.clone();

    // Dispatch to handler
    let result = dispatch_method(&request, db, channel_manager, broadcaster).await;

    match result {
        Ok(value) => RpcResponse::success(id, value),
        Err(error) => RpcResponse::error(id, error),
    }
}

async fn dispatch_method(
    request: &RpcRequest,
    db: &Arc<Database>,
    channel_manager: &Arc<ChannelManager>,
    broadcaster: &Arc<EventBroadcaster>,
) -> Result<serde_json::Value, RpcError> {
    match request.method.as_str() {
        "ping" => methods::handle_ping().await,
        "status" => methods::handle_status(broadcaster.clone()).await,
        "channels.status" => methods::handle_channels_status(db.clone(), channel_manager.clone()).await,
        "channels.start" => {
            let params: ChannelIdParams = serde_json::from_value(request.params.clone())
                .map_err(|e| RpcError::invalid_params(format!("Invalid params: {}", e)))?;
            methods::handle_channels_start(params, db.clone(), channel_manager.clone()).await
        }
        "channels.stop" => {
            let params: ChannelIdParams = serde_json::from_value(request.params.clone())
                .map_err(|e| RpcError::invalid_params(format!("Invalid params: {}", e)))?;
            methods::handle_channels_stop(params, channel_manager.clone(), db.clone()).await
        }
        "channels.restart" => {
            let params: ChannelIdParams = serde_json::from_value(request.params.clone())
                .map_err(|e| RpcError::invalid_params(format!("Invalid params: {}", e)))?;
            methods::handle_channels_restart(params, db.clone(), channel_manager.clone()).await
        }
        _ => Err(RpcError::method_not_found()),
    }
}
