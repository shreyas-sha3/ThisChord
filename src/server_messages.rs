use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "dm_list")]
    DmList { users: Vec<String> },

    // Optional: Add this if you want to send a message format back to client
    #[serde(rename = "info")]
    Info { from: String, msg: String },

    // Optional: For error handling
    #[serde(rename = "error")]
    Error { msg: String },
}
