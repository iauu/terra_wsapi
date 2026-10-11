use std::fmt::format;
use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::Arc;
use axum::body::Body;
use axum::extract::{ConnectInfo, State, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::Json;
use axum::response::IntoResponse;
use axum::routing::any;
use axum_extra::TypedHeader;
use tokio::sync::RwLock;
use crate::{ServerRouter, ServerState};
use crate::models::TerraState;
use itertools::Itertools;


pub fn routes() -> ServerRouter {
    ServerRouter::new()
        .route("/", any(get_online))
}

#[axum::debug_handler]
async fn get_online(
    State(state): State<Arc<RwLock<ServerState>>>,
) -> impl IntoResponse {
    let state = state.read().await.update_watch.borrow().deref().clone();
    let parsed = match serde_json::from_value::<TerraState>(state) {
        Ok(v) => v,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Body::from(
                format!("Model from terra have changed significantly and the server model need to be updated to serve the result\nError:\n{:?}", e)
            ));
        }
    };
    (StatusCode::OK, Body::from(serde_json::to_string(&parsed.players.iter().map(|(k, v)| v).sorted_by(|a, b| a.id.cmp(&b.id)).collect::<Vec<_>>()).unwrap()))
}