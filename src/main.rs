use futures_util::{SinkExt, StreamExt};
mod schema;
mod decoder;
mod cursor;
mod consts;
mod event;
mod request;
mod routes;
mod models;

use schema::parse_schema;

use std::collections::HashMap;
use axum::{body::Bytes, extract::ws::WebSocketUpgrade, response::{IntoResponse, Response}, routing::any, Json, Router};
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
use std::ops::{ControlFlow, Deref};
use std::path::PathBuf;
use cfg_if::cfg_if;
use reqwest::Url;
use bytes;
use futures_util::stream::SplitSink;
use serde_json::json;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;
use tracing::log::warn;
use crate::cursor::{ByteCursor, ParseError};
use crate::decoder::Decoder;
use tokio::time::sleep;
use crate::event::{decode_chat_history, decode_inbound_message, decode_ping_check, encode_pong_check, InboundMessage, MessageKind, FORCE_TAKEOVER};

pub type ServerRouter = Router<Arc<RwLock<ServerState>>>;

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
    #[error("Disconnection")]
    Disconnection
}

struct ServerState {
    update_watch: tokio::sync::watch::Receiver<serde_json::Value>,
    chat_rx: tokio::sync::broadcast::Receiver<InboundMessage>,
}

impl ServerState {
    pub fn new(
        watch: &tokio::sync::watch::Receiver<serde_json::Value>,
        rx: tokio::sync::broadcast::Receiver<InboundMessage>,
    ) -> Self {
        Self {
            update_watch: watch.clone(),
            chat_rx: rx
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

pub struct WebsocketTaskState<'a> {
    pub(crate) jwt: String,
    pub(crate) tx_ws_change:  &'a mut watch::Sender<serde_json::Value>,
    pub(crate) tx_message: &'a mut tokio::sync::broadcast::Sender<InboundMessage>
}


async fn process_message(message: Message, state: &'_ WebsocketTaskState<'_>, sender: &'_ mut  SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>, decoder: &'_ mut Option<Decoder>) -> Result<(), WsError> {
    match message {
        Message::Binary(data) => {
            if data.is_empty() {
                return Ok(())
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

                    *decoder = Some(Decoder::new(
                        root_schema_id,
                        schema_data,
                    ));

                    sender
                        .send(Message::Binary(
                            bytes::Bytes::from(vec![0x0a])
                        ))
                        .await?;
                }
                0x0e | 0x0f => {
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
                    // tracing::debug!("{:?}", json);
                    let _ = state.tx_ws_change.send(json);

                }
                0x0d => {
                    match MessageKind::from_bytes(&data[1..])? {
                        MessageKind::PingCheck => {
                            if let Some(ping) =
                                decode_ping_check(&data)?
                            {
                                let pong =
                                    encode_pong_check(ping.id)?;

                                sender
                                    .send(Message::Binary(pong.into()))
                                    .await?;
                            }
                        },
                        MessageKind::Unknown(v) => {
                            tracing::warn!("Unexpected message kind: {}", v);
                        },
                        MessageKind::PongCheck => {},
                        MessageKind::SessionConflict => {
                            sender.send(Message::Binary(FORCE_TAKEOVER.as_slice().into())).await?;
                        },
                        MessageKind::ForceTakeover => {},
                        MessageKind::ChatMessage => {
                            if let Some(message) = decode_inbound_message(&data)? {
                                let _ = state.tx_message.send(message);
                            }
                        },
                        MessageKind::ChatHistory => {
                            if let Some(history) = decode_chat_history(&data)? {
                            }
                        }
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
            sender
                .send(Message::Pong(payload).into())
                .await?;
        }
        Message::Pong(_) => {}
        Message::Close(frame) => {
            println!(
                "websocket closed: {:?}",
                frame
            );
            return Err(WsError::Disconnection);
        }
        Message::Text(text) => {
            println!(
                "unexpected websocket text: {}",
                text
            );
        }

        _ => {}
    };
    Ok(())
}

async fn ws_task(
    state: &'_ WebsocketTaskState<'_>,
) -> Result<(), WsError> {
    let client = reqwest::Client::new();

    let result = client
        .post(
            "https://game.terra.hackclub.com/\
             matchmake/joinOrCreate/world"
        )
        .json(&json!({
            "token": state.jwt,
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
        process_message(message, &state, &mut write, &mut decoder).await?;
    }

    Ok(())
}


#[tokio::main]
async fn main() {
    let (mut tx_ws_change, rx_ws_change) = tokio::sync::watch::channel(serde_json::Value::Null);
    let (mut tx_message, rx_message) = tokio::sync::broadcast::channel::<InboundMessage>(1024);
    let state = Arc::new(RwLock::new(ServerState::new(&rx_ws_change, rx_message)));


    tokio::spawn(async move {
        let task_state = WebsocketTaskState {
            jwt: env::var("JWT").unwrap(),
            tx_ws_change: &mut tx_ws_change,
            tx_message: &mut tx_message,
        };
        loop {
            match ws_task(&task_state).await {
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
        .merge(routes::routes())
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



