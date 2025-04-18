use std::{collections::{HashMap}, sync::Arc};
use futures_util::{StreamExt, SinkExt};
use tokio::sync::{RwLock, Mutex, broadcast};
use warp::ws::{Message as WsMessage, WebSocket};
use serde_json::json;

use crate::db::{get_or_create_conversation, store_direct_message, load_direct_messages,fetch_dm_list,load_server_messages,mark_messages_as_read};
use crate::message::{ClientMessage};
use crate::db::{DmMessage,ServerMessage};

type TxMap = Arc<RwLock<HashMap<String, Arc<Mutex<futures_util::stream::SplitSink<WebSocket, WsMessage>>>>>>;

pub async fn handle_socket(
    ws: WebSocket,
    tx: broadcast::Sender<(String, String)>,
    users: TxMap,
    db: Arc<sqlx::PgPool>,
    username: String,
) {
    let (sender, mut receiver) = ws.split();
    let sender = Arc::new(Mutex::new(sender));
    let mut rx = tx.subscribe();

    // // Send server history
    // let rows = sqlx::query!(
    //     "SELECT sender, content, sent_at FROM server_messages ORDER BY sent_at DESC LIMIT 30"
    // )
    // .fetch_all(&*db)
    // .await
    // .unwrap_or_default();
    
    // for row in rows.iter().rev() {
    //     let msg = json!([
    //         row.sender,
    //         row.content,
    //         "server",
    //         null,row.sent_at.to_rfc3339()
    //     ])
    //     .to_string();
    //     sender.lock().await.send(WsMessage::text(msg)).await.ok();
    // }
    


    users.write().await.insert(username.clone(), sender.clone());

    let connection_message = json!(["SERVER", format!("{} has entered the chat.", username), "server", null]).to_string();
    tx.send((connection_message, "SERVER".to_string())).ok();

    let cloned_username = username.clone();
    //let sender_clone = sender.clone();
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

    while let Some(Ok(msg)) = receiver.next().await {
        if let Ok(text) = msg.to_str() {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                continue;
            }
            //auto pinging
            if trimmed == "ping" {
                let payload = json!({
                    "type": "pingcheck",
                    "manual": false
                }).to_string();
                sender.lock().await.send(WsMessage::text(payload)).await.ok();
                continue;
            }
            match serde_json::from_str::<ClientMessage>(trimmed) {
            //SERVER RELATED
            //LOADING SERVER MESSAGES
            Ok(ClientMessage::LoadServerHistory { timestamp }) => {
                match load_server_messages(&db, 30, timestamp).await {
                    Ok(messages) => {
                        let sender_clone = sender.clone();
                        let is_history = timestamp.is_some();
            
                        if is_history {
                            //OLD MESSAGES (one array of 30 msgs)                                    
                            let messages_array: Vec<_> = messages
                                .into_iter()
                                .map(|ServerMessage { sender, content, sent_at, .. }| {
                                    json!([sender, content, "server", "all", sent_at.to_rfc3339(),true])
                                })
                                .collect();
            
                            let payload = json!({
                                "type": "load_prev_msgs",
                                "msgs": messages_array
                            })
                            .to_string();
            
                            sender_clone.lock().await.send(WsMessage::text(payload)).await.ok();
                        } else {
                            //NEW MESSAGES (one at a time)                                    
                            for ServerMessage { sender, content, sent_at, .. } in messages {
                                let server_msg = json!([sender, content, "server", "all", sent_at.to_rfc3339()])
                                    .to_string();
                                sender_clone.lock().await.send(WsMessage::text(server_msg)).await.ok();
                            }
                        }
                    }
                    Err(e) => {
                        let err_msg = json!([
                            "SERVER",
                            format!("Failed to load chat history: {}", e),
                            "server",
                            null
                        ])
                        .to_string();
            
                        let sender_err = sender.clone();
                        sender_err.lock().await.send(WsMessage::text(err_msg)).await.ok();
                    }
                }
            }

            Ok(ClientMessage::ServerMessage { msg }) => {
                match msg.as_str() {
                    "/users" => {
                        let user_list = users.read().await
                            .keys()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(", ");
                        let user_msg = json!(["SERVER", format!("Online users: {}", user_list), "server", null]).to_string();
                        sender.lock().await.send(WsMessage::text(user_msg)).await.ok();
                    }
                    "/ping" => {
                        let payload = json!({
                            "type": "pingcheck",
                            "manual": true
                        })
                        .to_string();
                        sender.lock().await.send(WsMessage::text(payload)).await.ok();
                    }
                    ":q" => {
                        let exit_msg = json!(["SERVER", "You have disconnected successfully.", "server", null]).to_string();
                        sender.lock().await.send(WsMessage::text(exit_msg)).await.ok();
                        users.write().await.remove(&username);
                        sender.lock().await.close().await.ok();
                        break;
                    }

                    _ => {
                        let json_msg = json!([username, msg, "server", null]).to_string();
                        let db = db.clone(); // Assuming you have access to sqlx::PgPool
                        let sender_clone = username.clone();
                        let msg_clone = msg.clone();
                        tokio::spawn(async move {
                            sqlx::query!(
                                "INSERT INTO server_messages (sender, content) VALUES ($1, $2)",
                                sender_clone,
                                msg_clone
                            )
                            .execute(&*db)
                            .await
                            .ok(); // optionally log on error
                        });
                        tx.send((json_msg, username.clone())).ok();

                    }
                }
            }

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
            
            //MARK MESSAGES AS READ
            Ok(ClientMessage::MarkDmRead { with }) => {
                if let Ok(conversation_id) = get_or_create_conversation(&db, &username, &with).await {
                    if let Err(e) = mark_messages_as_read(&db, conversation_id, &with).await {
                        eprintln!("Failed to mark messages as read: {}", e);
                    }
                }
            }
            

            //GET/CREATE DM CONVO ID
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

            //LOADING DM MESSAGES USING CONVO ID
            Ok(ClientMessage::LoadDmHistory { with, timestamp }) => {
                println!("TIME {:?} ", timestamp);
                match get_or_create_conversation(&db, &username, &with).await {
                    Ok(conversation_id) => {
                        match load_direct_messages(&db, conversation_id, 30, timestamp).await {
                            Ok(messages) => {
                                let sender_dm = sender_for_dm.clone();
                                let is_history = timestamp.is_some();
                                // LOADING OLD DMS
                                if is_history {
                                    let messages_array: Vec<_> = messages
                                        .into_iter()
                                        .map(|DmMessage { sender, content, sent_at, read}| {
                                            json!([sender, content, "dm", with, sent_at.to_rfc3339(),read])
                                        })
                                        .collect();
                                    
                                    // CREATING AN ARRAY OF MSGS
                                    let payload = json!({
                                        "type": "load_prev_msgs",
                                        "msgs": messages_array
                                    }).to_string();
                                    sender_dm.lock().await.send(WsMessage::text(payload)).await.ok();
                                } 
                                //NEW MESSAGES                                    
                                else {
                                    for DmMessage { sender, content, sent_at, read} in messages {
                                        let payload = json!([sender, content, "dm", with, sent_at.to_rfc3339(),read])
                                            .to_string();
                                        sender_dm.lock().await.send(WsMessage::text(payload)).await.ok();
                                    }
                                }
                            }
                            Err(e) => {
                                let err_msg = json!([
                                    "SERVER",
                                    format!("Failed to load chat history: {}", e),
                                    "server",
                                    null
                                ])
                                .to_string();
                                let sender_err = sender.clone();
                                sender_err.lock().await.send(WsMessage::text(err_msg)).await.ok();
                            }
                        }
                    }
                    Err(e) => {
                        let err_msg = json!([
                            "SERVER",
                            format!("Failed to get conversation: {}", e),
                            "server",
                            null
                        ])
                        .to_string();
                        let sender_err = sender.clone();
                        sender_err.lock().await.send(WsMessage::text(err_msg)).await.ok();
                    }
                }
            }

            Err(_) => {
                let warn_msg = json!(["SERVER", "Invalid message format.", "server", null]).to_string();
                sender.lock().await.send(WsMessage::text(warn_msg)).await.ok();
            }
        }
    }
}

    println!("{} disconnected", username);
    users.write().await.remove(&username);
    let disconnect_message = json!(["SERVER", format!("{} has left the chat.", username), "server", null]).to_string();
    tx.send((disconnect_message, "SERVER".to_string())).ok();
}
