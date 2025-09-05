use actix::{Actor, ActorContext, AsyncContext, Handler, Message, StreamHandler};
use actix_web::{Error, HttpRequest, HttpResponse, web};
use actix_web_actors::ws;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use uuid::Uuid;

use crate::worker::{JobQueue, JobStatus};

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

/// WebSocket message types
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum WsMessage {
    /// Subscribe to job status updates
    Subscribe { job_id: String },
    /// Unsubscribe from job status updates
    Unsubscribe { job_id: String },
    /// Job status update notification
    JobStatusUpdate { job_id: String, status: JobStatus },
    /// Heartbeat ping
    Ping,
    /// Heartbeat pong
    Pong,
    /// Error message
    Error { message: String },
}

/// WebSocket connection state
pub struct WsConnection {
    /// Unique connection ID
    id: String,
    /// Client must send ping at least once per 10 seconds (CLIENT_TIMEOUT),
    /// otherwise we drop connection.
    hb: Instant,
    /// Job IDs this connection is subscribed to
    subscriptions: Vec<String>,
    /// Reference to the connection manager
    manager: Arc<Mutex<ConnectionManager>>,
    /// Job queue for status checking
    job_queue: JobQueue,
}

impl WsConnection {
    pub fn new(manager: Arc<Mutex<ConnectionManager>>, job_queue: JobQueue) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            hb: Instant::now(),
            subscriptions: Vec::new(),
            manager,
            job_queue,
        }
    }

    /// Helper method to start heartbeat process
    fn hb(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // Check client heartbeats
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                // Heartbeat timed out
                log::warn!("WebSocket Client heartbeat failed, disconnecting!");
                ctx.stop();
                return;
            }

            ctx.ping(b"");
        });
    }

    /// Send job status update to client
    fn send_job_status(&self, ctx: &mut <Self as Actor>::Context, job_id: &str) {
        let job_queue = self.job_queue.clone();
        let job_id = job_id.to_string();

        let fut = async move {
            match job_queue.get_job_status(&job_id).await {
                Ok(Some(status)) => Some(WsMessage::JobStatusUpdate { job_id, status }),
                Ok(None) => Some(WsMessage::Error {
                    message: format!("Job {} not found", job_id),
                }),
                Err(e) => Some(WsMessage::Error {
                    message: format!("Failed to get job status: {}", e),
                }),
            }
        };

        let addr = ctx.address();
        actix::spawn(async move {
            if let Some(msg) = fut.await {
                addr.do_send(SendMessage(msg));
            }
        });
    }
}

impl Actor for WsConnection {
    type Context = ws::WebsocketContext<Self>;

    /// Method is called on actor start. We start the heartbeat process here.
    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);

        // Register this connection with the manager
        if let Ok(mut manager) = self.manager.lock() {
            manager.add_connection(self.id.clone(), ctx.address());
        }

        log::info!("WebSocket connection {} started", self.id);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> actix::Running {
        // Unregister this connection from the manager
        if let Ok(mut manager) = self.manager.lock() {
            manager.remove_connection(&self.id);
        }

        log::info!("WebSocket connection {} stopped", self.id);
        actix::Running::Stop
    }
}

/// Message to send to WebSocket client
#[derive(Message)]
#[rtype(result = "()")]
pub struct SendMessage(pub WsMessage);

impl Handler<SendMessage> for WsConnection {
    type Result = ();

    fn handle(&mut self, msg: SendMessage, ctx: &mut Self::Context) {
        if let Ok(text) = serde_json::to_string(&msg.0) {
            ctx.text(text);
        }
    }
}

/// Raw message to send to WebSocket client
#[derive(Message)]
#[rtype(result = "()")]
pub struct SendRawMessage(pub String);

impl Handler<SendRawMessage> for WsConnection {
    type Result = ();

    fn handle(&mut self, msg: SendRawMessage, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

/// WebSocket message handler
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsConnection {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let msg = match msg {
            Err(_) => {
                ctx.stop();
                return;
            }
            Ok(msg) => msg,
        };

        match msg {
            ws::Message::Ping(msg) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => {
                self.hb = Instant::now();
            }
            ws::Message::Text(text) => {
                self.hb = Instant::now();

                // Parse incoming message
                match serde_json::from_str::<WsMessage>(&text) {
                    Ok(WsMessage::Subscribe { job_id }) => {
                        log::info!("Connection {} subscribing to job {}", self.id, job_id);

                        // Add to subscriptions if not already present
                        if !self.subscriptions.contains(&job_id) {
                            self.subscriptions.push(job_id.clone());
                        }

                        // Send current job status
                        self.send_job_status(ctx, &job_id);
                    }
                    Ok(WsMessage::Unsubscribe { job_id }) => {
                        log::info!("Connection {} unsubscribing from job {}", self.id, job_id);
                        self.subscriptions.retain(|id| id != &job_id);
                    }
                    Ok(WsMessage::Ping) => {
                        let pong = WsMessage::Pong;
                        if let Ok(response) = serde_json::to_string(&pong) {
                            ctx.text(response);
                        }
                    }
                    Ok(_) => {
                        // Ignore other message types from client
                    }
                    Err(e) => {
                        log::warn!("Failed to parse WebSocket message: {}", e);
                        let error_msg = WsMessage::Error {
                            message: "Invalid message format".to_string(),
                        };
                        if let Ok(response) = serde_json::to_string(&error_msg) {
                            ctx.text(response);
                        }
                    }
                }
            }
            ws::Message::Binary(_) => {
                log::warn!("Unexpected binary message");
            }
            ws::Message::Close(reason) => {
                ctx.close(reason);
                ctx.stop();
            }
            ws::Message::Continuation(_) => {
                ctx.stop();
            }
            ws::Message::Nop => (),
        }
    }
}

/// Connection manager to track active WebSocket connections
pub struct ConnectionManager {
    connections: HashMap<String, actix::Addr<WsConnection>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
        }
    }

    pub fn add_connection(&mut self, id: String, addr: actix::Addr<WsConnection>) {
        self.connections.insert(id, addr);
    }

    pub fn remove_connection(&mut self, id: &str) {
        self.connections.remove(id);
    }

    pub fn broadcast_job_update(&self, job_id: &str, status: JobStatus) {
        // Send the generic job status update message
        let message = WsMessage::JobStatusUpdate {
            job_id: job_id.to_string(),
            status: status.clone(),
        };

        for addr in self.connections.values() {
            addr.do_send(SendMessage(message.clone()));
        }

        // Also send specific status-based messages for better frontend handling
        let specific_message = match status.status.as_str() {
            "pending" => Some(serde_json::json!({
                "type": "job_pending",
                "job_id": job_id,
                "status": {
                    "status": "pending",
                    "created_at": status.created_at,
                    "updated_at": status.updated_at
                }
            })),
            "processing" => Some(serde_json::json!({
                "type": "job_processing",
                "job_id": job_id,
                "status": {
                    "status": "processing",
                    "created_at": status.created_at,
                    "updated_at": status.updated_at
                }
            })),
            "completed" => Some(serde_json::json!({
                "type": "job_completed",
                "job_id": job_id,
                "status": {
                    "status": "completed",
                    "created_at": status.created_at,
                    "updated_at": status.updated_at
                }
            })),
            "failed" => Some(serde_json::json!({
                "type": "job_failed",
                "job_id": job_id,
                "error": status.error_message.as_ref().unwrap_or(&"Unknown error".to_string()).clone(),
                "status": {
                    "status": "failed",
                    "created_at": status.created_at,
                    "updated_at": status.updated_at,
                    "error_message": status.error_message.clone()
                }
            })),
            _ => None,
        };

        // Send the specific message as raw JSON to match frontend expectations
        if let Some(json_msg) = specific_message {
            if let Ok(text) = serde_json::to_string(&json_msg) {
                for addr in self.connections.values() {
                    // Send raw text message instead of structured WsMessage
                    addr.do_send(SendRawMessage(text.clone()));
                }
            }
        }
    }
}

/// WebSocket route handler
pub async fn websocket_handler(
    req: HttpRequest,
    stream: web::Payload,
    manager: web::Data<Arc<Mutex<ConnectionManager>>>,
    job_queue: web::Data<JobQueue>,
) -> Result<HttpResponse, Error> {
    let ws_conn = WsConnection::new(manager.get_ref().clone(), job_queue.get_ref().clone());
    ws::start(ws_conn, &req, stream)
}
