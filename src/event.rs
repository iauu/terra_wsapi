use std::str::FromStr;
use serde::{Deserialize, Serialize};
use crate::WsError;

#[derive(Debug, Deserialize)]
pub struct PingCheck {
    pub(crate) id: u64,
    pub rtt: f64,
}

#[derive(Debug, Deserialize)]
pub enum Channel {
    #[serde(rename = "global")]
    Global,
    #[serde(rename = "local")]
    Local
}

#[derive(Debug, Deserialize)]
pub enum Source {
    #[serde(rename = "terra")]
    Terra,
    #[serde(rename = "slack")]
    Slack
}

#[derive(Debug, Deserialize)]
pub struct InboundMessage {
    pub id: String,
    pub channel: Channel,
    pub username: String,
    pub displayName: String,
    pub source: Source,
    pub text: String,
    #[serde(rename = "createdAt", with = "time::serde::iso8601")]
    pub created_at: time::OffsetDateTime,
}

#[derive(Debug, Serialize)]
pub struct PongCheck {
    pub id: u64,
}

pub fn decode_ping_check(
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

pub fn encode_pong_check(
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

pub fn decode_inbound_message(data: &[u8]) -> Result<Option<InboundMessage>, WsError> {
    if data.first() != Some(&0x0d) {
        return Ok(None);
    }

    let mut de =
        rmp_serde::Deserializer::new(&data[1..]);

    let message_type =
        String::deserialize(&mut de)?;

    if message_type != "chat.message" {
        return Ok(None);
    }

    let ping =
        InboundMessage::deserialize(&mut de)?;

    Ok(Some(ping))
}

macro_rules! message_kind {
    ($(($name:ident => $bname:literal)),* $(,)?) => {
        pub enum MessageKind {
            $($name),*,
            Unknown(String)
        }

        impl ::std::str::FromStr for MessageKind {
            type Err = ::std::convert::Infallible;
            fn from_str(s: &str) -> ::std::result::Result<Self, Self::Err> {
                match s {
                    $($bname => Ok(MessageKind::$name),)*
                    v @ _ => Ok(MessageKind::Unknown(v.to_string()))
                }
            }
        }
    };
}

impl MessageKind {
    pub fn from_bytes(data: &[u8]) -> Result<MessageKind, WsError> {
        let mut de = rmp_serde::Deserializer::new(data);
        let message_type = String::deserialize(&mut de)?;
        return Ok(MessageKind::from_str(&message_type).unwrap());
    }
}

pub const FORCE_TAKEOVER: [u8; 19] = [0x0D, 0xAD, 0x66, 0x6F, 0x72, 0x63, 0x65, 0x54, 0x61, 0x6B, 0x65, 0x6F, 0x76, 0x65, 0x72, 0xD4, 0x72, 0x40, 0x90];

message_kind! {
    (PingCheck => "pingCheck"),
    (PongCheck => "pongCheck"),
    (SessionConflict => "sessionConflict"),
    (ForceTakeover => "forceTakeover"),
    (ChatMessage => "chat.message"),
}