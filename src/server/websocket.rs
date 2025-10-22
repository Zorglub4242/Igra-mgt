/// WebSocket handlers for real-time updates

use axum::{
    extract::{
        Path,
        WebSocketUpgrade,
    },
    extract::ws::{Message, WebSocket},
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use tokio::time::{interval, Duration};

use crate::core::DockerManager;

/// WebSocket handler for real-time log streaming
pub async fn ws_logs_handler(
    Path(service): Path<String>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| handle_logs_websocket(socket, service))
}

async fn handle_logs_websocket(socket: WebSocket, service: String) {
    let (mut sender, mut receiver) = socket.split();

    let mut interval = interval(Duration::from_millis(250));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                // Get latest logs
                if let Ok(docker) = DockerManager::new().await {
                    if let Ok(logs) = docker.get_logs(&service, Some(100)).await {
                        // Split logs into lines for JSON array
                        let lines: Vec<String> = logs.lines().map(|s| s.to_string()).collect();

                        if let Ok(json) = serde_json::to_string(&lines) {
                            if sender.send(Message::Text(json)).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            }

            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(data))) => {
                        if sender.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

/// WebSocket handler for real-time metrics streaming
pub async fn ws_metrics_handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_metrics_websocket)
}

use serde::Serialize;

#[derive(Serialize)]
struct MetricsSnapshot {
    timestamp: i64,
    services: Vec<ServiceMetrics>,
}

#[derive(Serialize)]
struct ServiceMetrics {
    name: String,
    cpu_percent: f64,
    memory_mb: f64,
    status: String,
}

async fn handle_metrics_websocket(socket: WebSocket) {
    let (mut sender, mut receiver) = socket.split();

    let mut interval = interval(Duration::from_secs(2));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                if let Ok(docker) = DockerManager::new().await {
                    if let Ok(containers) = docker.list_containers().await {
                        let mut services = Vec::new();

                        for c in containers {
                            let stats = docker.get_container_stats(&c.name).await.ok().flatten();

                            let (cpu_percent, memory_mb) = if let Some(s) = stats {
                                (s.cpu_percent, s.memory_usage as f64 / 1024.0 / 1024.0)
                            } else {
                                (0.0, 0.0)
                            };

                            services.push(ServiceMetrics {
                                name: c.name,
                                cpu_percent,
                                memory_mb,
                                status: c.status,
                            });
                        }

                        let snapshot = MetricsSnapshot {
                            timestamp: chrono::Utc::now().timestamp(),
                            services,
                        };

                        if let Ok(json) = serde_json::to_string(&snapshot) {
                            if sender.send(Message::Text(json)).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            }

            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(data))) => {
                        if sender.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
