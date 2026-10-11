use std::net::SocketAddr;
use std::sync::Arc;
use axum::extract::{ConnectInfo, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::any;
use axum_extra::TypedHeader;
use tokio::sync::RwLock;
use crate::routes::{chat, ws, ws_high};
use crate::{ServerRouter, ServerState};

pub fn routes() -> ServerRouter {
    ServerRouter::new()
        .route("/", any(ws_output_chat))
}

async fn ws_output_chat(
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<RwLock<ServerState>>>,
) -> impl IntoResponse {
    let mut chat_rx = state.read().await.chat_rx.resubscribe();
    ws.on_upgrade(|mut socket| async move {
        while let Ok(message) = chat_rx.recv().await {
            let value = serde_json::to_string(&message).unwrap();
            let _ = socket.send(axum::extract::ws::Message::Text(value.into())).await;
        }
    })
}