use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};


#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "dm")]
    DirectMessage { to: String, msg: String },

    #[serde(rename = "server")]
    ServerMessage { msg: String },

    #[serde(rename = "load_server")]
    LoadServerHistory {timestamp: Option<DateTime<Utc>>},

    #[serde(rename = "load_dm")]
    LoadDmHistory { with: String ,timestamp: Option<DateTime<Utc>>},

    #[serde(rename = "mark_read")]
    MarkDmRead { with: String },
    
    #[serde(rename = "fetch_dm_list")]
    FetchDmList, 
}
