use futures_util::{SinkExt, StreamExt};
mod schema;
mod decoder;
mod cursor;
mod consts;

use schema::parse_schema;

use std::collections::HashMap;
use axum::{body::Bytes, extract::ws::{WebSocketUpgrade}, response::{IntoResponse, Response}, routing::any, Json, Router};
use axum_extra::TypedHeader;
use indexmap::IndexMap;
use axum::body::Body;
use axum::extract::ws::Utf8Bytes;
use axum::extract::{ConnectInfo, Query, State};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::{watch, Mutex, RwLock};
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use ahash::RandomState;
use rand::Rng;

use std::{env, thread};
use std::fmt::format;
use std::ops::Deref;
use std::path::PathBuf;
use cfg_if::cfg_if;
use reqwest::Url;
use bytes;
use serde_json::json;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;
use tracing::log::warn;
use crate::cursor::{ByteCursor, ParseError};
use crate::decoder::Decoder;
use tokio::time::sleep;


#[derive(Debug, Error)]
pub enum WsError {
    #[error(transparent)]
    Parse(#[from] ParseError),

    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    WebSocket(
        #[from]
        tokio_tungstenite::tungstenite::Error
    ),

    #[error(transparent)]
    MsgPackDecode(
        #[from]
        rmp_serde::decode::Error
    ),

    #[error(transparent)]
    MsgPackEncode(
        #[from]
        rmp_serde::encode::Error
    ),
}

struct ServerState {
    broadcast: tokio::sync::watch::Receiver<serde_json::Value>,
}

impl ServerState {
    pub fn new(recv: &tokio::sync::watch::Receiver<serde_json::Value>) -> Self {
        Self {
            broadcast: recv.clone(),
        }
    }
}

#[derive(Deserialize)]
pub struct MatchMakeResp {
    name: String,
    sessionId: String,
    roomId: String,
    processId: String,
}

#[derive(Debug, Deserialize)]
struct PingCheck {
    id: u64,
    rtt: f64,
}

#[derive(Debug, Serialize)]
struct PongCheck {
    id: u64,
}

fn decode_ping_check(
    data: &[u8],
) -> Result<Option<PingCheck>, WsError> {
    if data.first() != Some(&0x0d) {
        return Ok(None);
    }

    let mut de =
        rmp_serde::Deserializer::new(&data[1..]);

    let message_type =
        String::deserialize(&mut de)?;

    if message_type != "pingCheck" {
        return Ok(None);
    }

    let ping =
        PingCheck::deserialize(&mut de)?;

    Ok(Some(ping))
}

fn encode_pong_check(
    id: u64,
) -> Result<Vec<u8>, WsError> {
    let mut out = Vec::new();
    out.push(0x0d);

    rmp_serde::encode::write(
        &mut out,
        &"pongCheck",
    )?;
    rmp_serde::encode::write(
        &mut out,
        &PongCheck { id },
    )?;

    Ok(out)
}

async fn ws_task(
    jwt: String,
    tx: &mut watch::Sender<serde_json::Value>,
) -> Result<(), WsError> {
    let client = reqwest::Client::new();

    let result = client
        .post(
            "https://game.terra.hackclub.com/\
             matchmake/joinOrCreate/world"
        )
        .json(&json!({
            "token": jwt,
            "protocol": 1,
            "levelId":"town-square",
        }))
        .send()
        .await?
        .error_for_status()?;

    let resp: MatchMakeResp = result.json().await?;
    let ws_url = format!(
        "wss://game.terra.hackclub.com/{}/{}?sessionId={}",
        &resp.processId,
        &resp.roomId,
        &resp.sessionId,
    );

    tracing::info!("Connecting to {}", &ws_url);

    let request = ws_url.into_client_request()?;

    let (ws, _) = connect_async(request).await?;

    let (mut write, mut read) = ws.split();

    let mut decoder: Option<Decoder> = None;

    while let Some(message) = read.next().await {
        let message = message?;

        match message {
            Message::Binary(data) => {
                if data.is_empty() {
                    continue;
                }

                match data[0] {
                    0x0a => {
                        tracing::debug!(
                            "JOIN/SCHEMA: {} bytes",
                            data.len()
                        );
                        let schema_data = match parse_schema(&data) {
                            Ok(v) => v,
                            Err(e) => {
                                tracing::error!(
                                    "parse_schema failed: {:?}",
                                e);
                                return Err(e.into());
                            }
                        };
                        tracing::debug!(
                        "root_schema_id={:?}, schema_ids={:?}",
                        schema_data.root_schema_id,
                        schema_data.schema_entries.keys().collect::<Vec<_>>(),
                    );

                                        tracing::debug!(
                        "root field 0={:?}",
                        schema_data.get_schema_field_type(
                            &schema_data.root_schema_id,
                            0,
                        )
                    );

                        let root_schema_id =
                            schema_data.root_schema_id;

                        decoder = Some(Decoder::new(
                            root_schema_id,
                            schema_data,
                        ));

                        write
                            .send(Message::Binary(
                                bytes::Bytes::from(vec![0x0a])
                            ))
                            .await?;
                    }
                    0x0e => {
                        let decoder = decoder
                            .as_mut()
                            .ok_or(
                                ParseError::InvalidSchema("Unable to obtain decoder")
                            )?;

                        let mut cursor =
                            ByteCursor::new(
                                data.as_ref()
                            );

                        decoder.apply(&mut cursor)?;
                        let json = decoder.to_json()?;
                        tracing::debug!("{:?}", json);
                        let _ = tx.send(json);

                    }
                    0x0f => {
                        let decoder = decoder
                            .as_mut()
                            .ok_or(
                                ParseError::InvalidSchema("Unable to obtain decoder")
                            )?;

                        let mut cursor =
                            ByteCursor::new(
                                data.as_ref()
                            );

                        decoder.apply(&mut cursor)?;
                        let json = decoder.to_json()?;
                        tracing::debug!("{:?}", json);
                        let _ = tx.send(json);
                    }
                    0x0d => {
                        if let Some(ping) =
                            decode_ping_check(&data)?
                        {
                            let pong =
                                encode_pong_check(ping.id)?;

                            write
                                .send(Message::Binary(pong.into()).into())
                                .await?;
                        }
                    }
                    opcode => {
                        eprintln!(
                            "unknown binary opcode: \
                             0x{opcode:02x}"
                        );
                    }
                }
            }
            Message::Ping(payload) => {
                write
                    .send(Message::Pong(payload).into())
                    .await?;
            }
            Message::Pong(_) => {}
            Message::Close(frame) => {
                println!(
                    "websocket closed: {:?}",
                    frame
                );
                break;
            }
            Message::Text(text) => {
                println!(
                    "unexpected websocket text: {}",
                    text
                );
            }

            _ => {}
        }
    }

    Ok(())
}

async fn ws_output(
    ws: WebSocketUpgrade,
    _user_agent: Option<TypedHeader<headers::UserAgent>>,
    ConnectInfo(_addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<RwLock<ServerState>>>,
) -> impl IntoResponse {
    let mut watch_rx = state.read().await.broadcast.clone();
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


#[tokio::main]
async fn main() {
    let (mut tx, rx) = tokio::sync::watch::channel(serde_json::Value::Null);
    let state = Arc::new(RwLock::new(ServerState::new(&rx)));


    tokio::spawn(async move {
        loop {
            match ws_task(env::var("JWT").unwrap(), &mut tx).await {
                Ok(()) => {},
                Err(e) => {
                    tracing::error!("Websocket connection error: {}", e);
                }
            }
            sleep(Duration::from_secs(5)).await;
        }
    });


    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();


    let _clone = state.clone();
    let app = Router::new()
        .route("/ws", any(ws_output))
        .with_state(_clone)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        );
    let listener = tokio::net::TcpListener::bind("[::]:3000").await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    let _ = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
        .await;
}



