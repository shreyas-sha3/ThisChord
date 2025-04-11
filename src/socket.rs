use std::{collections::{HashMap, VecDeque}, sync::Arc};
use futures_util::{StreamExt, SinkExt};
use tokio::{sync::{RwLock, Mutex, broadcast}, spawn};
use warp::ws::{Message as WsMessage, WebSocket};
use serde_json::json;
use chrono::Utc;

use crate::db::{get_or_create_conversation, store_direct_message, load_direct_messages};
use crate::message::{ClientMessage};
use crate::db::ChatMessage;
use crate::db::fetch_dm_list;

type TxMap = Arc<RwLock<HashMap<String, Arc<Mutex<futures_util::stream::SplitSink<WebSocket, WsMessage>>>>>>;

pub async fn handle_socket(
    ws: WebSocket,
    tx: broadcast::Sender<(String, String)>,
    users: TxMap,
    message_history: Arc<Mutex<VecDeque<(String, String, String, Option<String>)>>>,
    db: Arc<sqlx::PgPool>,
    username: String,
) {
    let (sender, mut receiver) = ws.split();
    let sender = Arc::new(Mutex::new(sender));
    let mut rx = tx.subscribe();

    // Send broadcast history
    {
        let history = message_history.lock().await;
        for (sender_name, msg, msg_type, target) in history.iter() {
            let json_msg = json!([sender_name, msg, msg_type, target]).to_string();
            sender.lock().await.send(WsMessage::text(json_msg)).await.ok();
        }
        
    }

    users.write().await.insert(username.clone(), sender.clone());

    let connection_message = json!(["SERVER", format!("{} has entered the chat.", username), "broadcast", null]).to_string();
    tx.send((connection_message, "SERVER".to_string())).ok();

    let cloned_username = username.clone();
    let sender_clone = sender.clone();
    let users_clone = users.clone();

    let sender_clone = sender.clone();
    let sender_for_dm = sender.clone(); 
    
    tokio::spawn(async move {
        while let Ok((msg, _sender_name)) = rx.recv().await {
            let users_lock = users_clone.read().await;
            if users_lock.contains_key(&cloned_username) {
                sender_clone.lock().await.send(WsMessage::text(msg)).await.ok();
            }
        }
    });
    

    let history_clone = message_history.clone();

    while let Some(Ok(msg)) = receiver.next().await {
        if let Ok(text) = msg.to_str() {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                continue;
            }

            match serde_json::from_str::<ClientMessage>(trimmed) {
                Ok(ClientMessage::FetchDmList) => {
                    let dm_list = fetch_dm_list(&db, &username).await.unwrap_or_default();
                    let json = match serde_json::to_string(&serde_json::json!({
                        "type": "dm_list",
                        "users": dm_list
                    })) {
                        Ok(s) => s,
                        Err(e) => {
                            eprintln!("Failed to serialize DM list: {}", e);
                            continue;
                        }
                    };
                    sender.lock().await.send(WsMessage::text(json)).await.ok();                    
                    
                }
                Ok(ClientMessage::DirectMessage { to, msg }) => {
                    let conversation_id = get_or_create_conversation(&db, &username, &to).await.unwrap();
                    store_direct_message(&db, conversation_id, &username, &msg).await.unwrap();

                    let dm_text = json!([username, msg, "dm", to]).to_string();
                    let users_map = users.read().await;

                    if let Some(recipient_sender) = users_map.get(&to) {
                        recipient_sender.lock().await.send(WsMessage::text(dm_text.clone())).await.ok();
                    }
                    sender.lock().await.send(WsMessage::text(dm_text)).await.ok();
                }
                Ok(ClientMessage::LoadDmHistory { with }) => {
                    match get_or_create_conversation(&db, &username, &with).await {
                        Ok(conversation_id) => {
                            match load_direct_messages(&db, conversation_id, 100).await {
                                Ok(messages) => {
                                    let sender_dm = sender_for_dm.clone(); 
                                    for ChatMessage { sender, content, sent_at, .. } in messages {
                                        let dm_msg = json!([sender, content, "dm", with, sent_at.to_rfc3339()]).to_string();
                                        sender_dm.lock().await.send(WsMessage::text(dm_msg)).await.ok();
                                    }
                                    
                                }
                                Err(e) => {
                                    let err_msg = json!(["SERVER", format!("Failed to load chat history: {}", e), "broadcast", null]).to_string();
                                    let sender_clone = sender.clone();
                                    let sender_err = sender_clone.clone(); 
                                    sender_err.lock().await.send(WsMessage::text(err_msg)).await.ok();
                                }
                            }
                        }
                        Err(e) => {
                            let err_msg = json!(["SERVER", format!("Failed to get conversation: {}", e), "broadcast", null]).to_string();
                            let sender_clone = sender.clone();
                            let sender_err = sender_clone.clone(); 
                            sender_err.lock().await.send(WsMessage::text(err_msg)).await.ok();
                        }
                    }
                }
                

                Ok(ClientMessage::BroadcastMessage { msg }) => {
                    match msg.as_str() {
                        "/users" => {
                            let user_list = users.read().await
                                .keys()
                                .cloned()
                                .collect::<Vec<_>>()
                                .join(", ");
                            let user_msg = json!(["SERVER", format!("Online users: {}", user_list), "broadcast", null]).to_string();
                            sender.lock().await.send(WsMessage::text(user_msg)).await.ok();
                        }

                        ":q" => {
                            let exit_msg = json!(["SERVER", "You have disconnected successfully.", "broadcast", null]).to_string();
                            sender.lock().await.send(WsMessage::text(exit_msg)).await.ok();
                            users.write().await.remove(&username);
                            sender.lock().await.close().await.ok();
                            break;
                        }

                        _ => {
                            let json_msg = json!([username, msg, "broadcast", null]).to_string();
                            {
                                let mut history = history_clone.lock().await;
                                if history.len() >= 30 {
                                    history.pop_front();
                                }
                                history.push_back((username.clone(), msg.clone(), "broadcast".to_string(), None));
                            }
                            tx.send((json_msg, username.clone())).ok();
                        }
                    }
                }

                Err(_) => {
                    let warn_msg = json!(["SERVER", "Invalid message format.", "broadcast", null]).to_string();
                    sender.lock().await.send(WsMessage::text(warn_msg)).await.ok();
                }
            }
        }
    }

    println!("{} disconnected", username);
    users.write().await.remove(&username);
    let disconnect_message = json!(["SERVER", format!("{} has left the chat.", username), "broadcast", null]).to_string();
    tx.send((disconnect_message, "SERVER".to_string())).ok();
}
