// src-tauri/src/commands/cdp_server.rs

use crate::cdp::{CdpMessage, CdpResponse};
use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};

pub struct CdpServer {
    port: u16,
}

impl CdpServer {
    pub fn new(port: u16) -> Self {
        Self { port }
    }

    /// Start the CDP server
    pub async fn start(&self) -> Result<(), String> {
        let addr: SocketAddr = ([127, 0, 0, 1], self.port).into();
        let listener = TcpListener::bind(&addr).await.map_err(|e| e.to_string())?;
        println!("CDP server listening on ws://{}", addr);
        let port = self.port;

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    println!("CDP client connected: {}", peer_addr);
                    tokio::spawn(Self::handle_connection(stream, port));
                }
                Err(e) => {
                    eprintln!("CDP accept error: {}", e);
                }
            }
        }
    }

    async fn handle_connection(stream: TcpStream, _port: u16) {
        let ws = match accept_async(stream).await {
            Ok(ws) => ws,
            Err(e) => {
                eprintln!("WebSocket handshake failed: {}", e);
                return;
            }
        };

        let (mut write, mut read) = ws.split();

        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Some(response) = Self::handle_text_message(&text) {
                        if write.send(Message::Text(response.into())).await.is_err() {
                            break;
                        }
                    }
                }
                Ok(Message::Close(_)) => break,
                Err(e) => {
                    eprintln!("CDP message error: {}", e);
                    break;
                }
                _ => {}
            }
        }
    }

    fn handle_text_message(text: &str) -> Option<String> {
        let msg: CdpMessage = serde_json::from_str(text).ok()?;
        let response = Self::dispatch(&msg)?;
        serde_json::to_string(&response)
            .map_err(|e| {
                eprintln!("CDP serialize error: {}", e);
            })
            .ok()
    }

    fn dispatch(msg: &CdpMessage) -> Option<CdpResponse> {
        let id = match msg.id {
            Some(id) => id,
            None => {
                eprintln!(
                    "CDP dispatch error: missing message id for method {}",
                    msg.method
                );
                return None;
            }
        };

        // Handle basic CDP methods
        match msg.method.as_str() {
            "Page.enable" | "Runtime.enable" | "Network.enable" => Some(CdpResponse {
                id,
                result: Some(serde_json::json!({})),
                error: None,
            }),
            "Page.disable" | "Runtime.disable" | "Network.disable" => Some(CdpResponse {
                id,
                result: Some(serde_json::json!({})),
                error: None,
            }),
            "Target.getTargets" => Some(CdpResponse {
                id,
                result: Some(serde_json::json!({
                    "targetInfos": []
                })),
                error: None,
            }),
            _ => {
                // Return empty success for unknown methods
                Some(CdpResponse {
                    id,
                    result: Some(serde_json::json!({})),
                    error: None,
                })
            }
        }
    }
}
