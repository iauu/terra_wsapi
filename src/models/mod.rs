use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NpcData {
    pub id: String,
    pub name: String,
    pub rotation: f64,
    pub scale: f64,
    pub sprite: String,
    pub x: f64,
    pub z: f64
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Player {
    #[serde(rename = "clothesHue")]
    pub clothes_hue: u16,
    #[serde(rename="displayName")]
    pub display_name: String,
    #[serde(rename = "equippedAccessories")]
    pub equipped_accessories: serde_json::Value,
    pub id: String,
    #[serde(rename="lastInputSeq")]
    pub last_input_seq: u16,
    #[serde(rename="lastProcessedSeq")]
    pub last_processed_seq: u16,
    #[serde(rename="profilePictureUrl")]
    pub profile_picture_url: String,
    #[serde(rename="slackId")]
    pub slack_id: String,
    pub username: String,
    #[serde(rename="velX")]
    pub vel_x: f64,
    #[serde(rename="velZ")]
    pub vel_z: f64,
    pub x: f64,
    pub z: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TerraState {
    pub farmItems: serde_json::Value,
    pub levelId: String,
    pub npcs: Vec<NpcData>,
    pub players: Vec<Player>,
    pub serverTime: u64
}