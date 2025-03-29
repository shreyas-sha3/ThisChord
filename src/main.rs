use std::env;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use warp::Filter;
use futures_util::{StreamExt, SinkExt};
use warp::ws::{WebSocket, Message as WsMessage};
use std::collections::HashMap;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() {
    let (tx, _rx) = broadcast::channel::<(String, String)>(10); // Broadcast messages with usernames
    let users = Arc::new(RwLock::new(HashMap::new()));
    let chat = warp::path("chat")
        .and(warp::ws())
        .map(move |ws: warp::ws::Ws| {
            let tx = tx.clone();
            let users = users.clone();
            ws.on_upgrade(move |socket| handle_socket(socket, tx,users))
        });

    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string()).parse().expect("Invalid PORT value");
    println!("WebSocket server running on ws://0.0.0.0:{}", port);

    warp::serve(chat).run(([0, 0, 0, 0], port)).await;
}

/// **Handles WebSocket Connection**
async fn handle_socket(ws: WebSocket, tx: broadcast::Sender<(String, String)>, users: Arc<RwLock<HashMap<String, Arc<Mutex<futures_util::stream::SplitSink<WebSocket, WsMessage>>>>>>) 
{
    let (sender, mut receiver) = ws.split();
    let sender = Arc::new(Mutex::new(sender)); // ✅ Wrap sender in Arc<Mutex<>> to allow shared access
    let mut rx = tx.subscribe();

    let mut username = String::new();

    // 🔹 Ask for username (force non-empty)
    loop {
        sender.lock().await.send(WsMessage::text("Enter your username:")).await.ok();
    
        if let Some(Ok(msg)) = receiver.next().await {
            if let Ok(text) = msg.to_str() {
                let trimmed = text.trim();
                let mut users_lock = users.write().await;
                if !trimmed.is_empty() && !users_lock.contains_key(trimmed) {  // ✅ Ensure username is unique
                    username = trimmed.to_string();
                    users_lock.insert(username.clone(), sender.clone());
                    break;
                } else {
                    sender.lock().await.send(WsMessage::text("Username Error (Invalid/Taken)")).await.ok();
                }
            }
        }
    }
    
    sender.lock().await.send(WsMessage::text(format!("Welcome, {}! Type :q to leave.", username))).await.ok();

    let cloned_username = username.clone();
    let sender_clone = sender.clone();

    // 🔹 Background task to receive messages
    tokio::spawn(async move {
        while let Ok((msg, sender_name)) = rx.recv().await {
            if sender_name != cloned_username {
                sender_clone.lock().await.send(WsMessage::text(msg)).await.ok();
            }
        }
    });

    // 🔹 Main chat loop
    while let Some(Ok(msg)) = receiver.next().await {
        if let Ok(text) = msg.to_str() {
            if text == ":q" {
                sender.lock().await.send(WsMessage::text("You have disconnected successfully.")).await.ok();
                tx.send((format!("{} has left the chat.", username), username.clone())).ok();
                println!("{} disconnected", username);
                users.write().await.remove(&username);
                break;
            }
            tx.send((format!("{}: {}", username, text), username.clone())).ok();
        }
    }
}
