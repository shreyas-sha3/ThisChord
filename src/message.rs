use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "dm")]
    DirectMessage { to: String, msg: String },

    #[serde(rename = "broadcast")]
    BroadcastMessage { msg: String },

    #[serde(rename = "load_dm")]
    LoadDmHistory { with: String },

    #[serde(rename = "fetch_dm_list")]
    FetchDmList,
}
