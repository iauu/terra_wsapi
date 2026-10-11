use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NpcData {
    id: String,
    name: String,
    rotation: f64,
    scale: f64,
    sprite: String,
    x: f64,
    z: f64
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Player {
    #[serde(rename = "clothesHue")]
    clothes_hue: u16,
    #[serde(rename="displayName")]
    display_name: String,
    #[serde(rename = "equippedAccessories")]
    equipped_accessories: serde_json::Value,
    id: String,
    #[serde(rename="lastInputSeq")]
    last_input_seq: u16,
    #[serde(rename="lastProcessedSeq")]
    last_processed_seq: u16,
    #[serde(rename="profilePictureUrl")]
    profile_picture_url: String,
    #[serde(rename="slackId")]
    slack_id: String,
    username: String,
    #[serde(rename="velX")]
    vel_x: f64,
    #[serde(rename="velZ")]
    vel_z: f64,
    x: f64,
    z: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ServerState {
    farmItems: serde_json::Value,
    levelId: String,
    npcs: Vec<NpcData>,
    players: Vec<Player>,
    serverTime: u64
}