use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::Arc;
use axum::body::Body;
use axum::extract::{ConnectInfo, State, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::Json;
use axum::response::IntoResponse;
use axum_extra::TypedHeader;
use tokio::sync::RwLock;
use crate::{ServerRouter, ServerState};
use crate::models::TerraState;


pub fn routes() -> ServerRouter {
    ServerRouter::new()
}

#[axum::debug_handler]
async fn get_online(
    State(state): State<Arc<RwLock<ServerState>>>,
) -> impl IntoResponse {
    let state = state.read().await.update_watch.borrow().deref().clone();
    let Ok(parsed) = serde_json::from_value::<TerraState>(state) else {
        return (StatusCode::INTERNAL_SERVER_ERROR, Body::from("Model from terra have changed significantly and the server model need to be updated to serve the result"));
    };
    (StatusCode::OK, Body::from(serde_json::to_string(&parsed.players).unwrap()))
}