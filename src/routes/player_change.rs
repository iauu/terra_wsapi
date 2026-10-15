use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;
use ahash::{HashMap, HashMapExt};
use axum::body::Body;
use axum::extract::{ConnectInfo, State, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use axum_extra::TypedHeader;
use futures_util::SinkExt;
use serde::{Deserialize, Serialize};
use serde_hashkey::to_key_with_ordered_float;
use tokio::sync::RwLock;
use tokio::time::sleep;
use crate::models::{Player, TerraState};
use crate::{ServerRouter, ServerState};

pub fn routes() -> ServerRouter {
    ServerRouter::new()
        .route("/", any(ws_output_player_change))
}

#[derive(Serialize, Deserialize)]
pub enum EventType {
    #[serde(rename = "initial")]
    Initial,
    #[serde(rename = "change")]
    Change,
}

#[derive(Serialize, Deserialize)]
pub struct PlayerChangeEvent {
    pub event_type: EventType,
    pub players: Vec<Player>,
}

async fn get_player_list(state: &Arc<RwLock<ServerState>>) -> Result<Vec<Player>, Response<Body>> {
    let state = state.read().await.update_watch.borrow().deref().clone();
    let parsed = match serde_json::from_value::<TerraState>(state) {
        Ok(v) => v,
        Err(e) => {
            return Err(Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(
                    format!("Model from terra have changed significantly and the server model need to be updated to serve the result\nError:\n{:?}", e)
                )).unwrap());
        }
    };
    Ok(parsed.players.into_iter().map(|x| x.1).collect::<Vec<_>>())
}

macro_rules! unwrap_return {
    ($x:expr) => {
        match $x {
            Ok(x) => x,
            Err(e) => {
                return e;
            }
        }
    };
}

macro_rules! send {
    ($socket:ident, $v:expr) => {
        {
            let value = serde_json::to_string(&$v).unwrap();
            let _ = $socket.send(axum::extract::ws::Message::Text(value.into())).await;
        }
    };
}

async fn ws_output_player_change(
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<RwLock<ServerState>>>,
) -> impl IntoResponse {
    let mut watch = state.read().await.update_watch.clone();

    let mut player_state = unwrap_return!(get_player_list(&state).await);
    let mut player_state_map = HashMap::new();
    for (i, v) in player_state.iter().enumerate() {
        player_state_map.insert(to_key_with_ordered_float(&v).unwrap(), i);
    }
    let mut overall_state = to_key_with_ordered_float(&player_state).unwrap();
    ws.on_upgrade(|mut socket| async move {
        send!(socket, PlayerChangeEvent {
            event_type: EventType::Initial,
            players: player_state.clone(),
        });
        while let Ok(change) = watch.has_changed() {
            if change {
                watch.mark_unchanged();
                let Ok(new_player_state) =  get_player_list(&state).await else {
                    let _ = socket.close().await;
                    return;
                };
                if overall_state == to_key_with_ordered_float(&new_player_state).unwrap() {
                    continue;
                }
                let mut new_map = HashMap::new();
                for (i, v) in new_player_state.iter().enumerate() {
                    new_map.insert(to_key_with_ordered_float(&v).unwrap(), i);
                };
                let changed = new_map
                    .iter()
                    .filter(
                        |(k, v)| !player_state_map.contains_key(&k)
                    )
                    .map(
                    |(k, v)| new_player_state.get(*v).unwrap().clone()
                    )
                    .collect::<Vec<_>>();
                send!(socket, PlayerChangeEvent {
                    event_type: EventType::Change,
                    players: changed
                });
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
}