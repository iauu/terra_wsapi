use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;
use axum::extract::{ConnectInfo, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum_extra::TypedHeader;
use tokio::sync::RwLock;
use tokio::time::sleep;
use crate::ServerState;

use axum::routing::any;
use crate::ServerRouter;

pub fn routes() -> ServerRouter {
    ServerRouter::new()
        .route("/", any(ws_output))
}

async fn ws_output(
    ws: WebSocketUpgrade,
    _user_agent: Option<TypedHeader<headers::UserAgent>>,
    ConnectInfo(_addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<RwLock<ServerState>>>,
) -> impl IntoResponse {
    let mut watch_rx = state.read().await.update_watch.clone();
    ws.on_upgrade(|mut socket| async move {
        while let Ok(change) = watch_rx.has_changed() {
            if change {
                let data = watch_rx.borrow().deref().clone();
                watch_rx.mark_unchanged();
                let _ = socket
                    .send(axum::extract::ws::Message::Text(serde_json::to_string(&data).unwrap().into())).await;
            }
            sleep(Duration::from_secs(5)).await;
        }
    })
}