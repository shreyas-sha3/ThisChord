use warp::Filter;
use std::env;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, RwLock};
use warp::ws::{WebSocket, Message as WsMessage};
use futures_util::{StreamExt, SinkExt};
use std::collections::{HashMap, VecDeque};

#[tokio::main]
async fn main() {
    let (tx, _rx) = broadcast::channel::<(String, String)>(10);
    let users = Arc::new(RwLock::new(HashMap::new()));
    let message_history = Arc::new(Mutex::new(VecDeque::new())); 
    let static_files = warp::fs::dir("static");

    let chat = warp::path("chat")
        .and(warp::ws())
        .map(move |ws: warp::ws::Ws| {
            let tx = tx.clone();
            let users = users.clone();
            let history = message_history.clone();  
            ws.on_upgrade(move |socket| handle_socket(socket, tx, users, history))
        });
    
    let routes = chat.or(static_files);

    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string()).parse().expect("Invalid PORT value");
    println!("WebSocket server running on ws://0.0.0.0:{}", port);

    warp::serve(routes).run(([0, 0, 0, 0], port)).await;
}

async fn handle_socket(
    ws: WebSocket, 
    tx: broadcast::Sender<(String, String)>, 
    users: Arc<RwLock<HashMap<String, Arc<Mutex<futures_util::stream::SplitSink<WebSocket, WsMessage>>>>>>,
    message_history: Arc<Mutex<VecDeque<String>>>, 
) {
    let (sender, mut receiver) = ws.split();
    let sender = Arc::new(Mutex::new(sender));
    let mut rx = tx.subscribe();

    let mut username = String::new();

    // Ask for username
    sender.lock().await.send(WsMessage::text(r#"["SERVER", "Enter your username"]"#)).await.ok();
    
    loop {    
        if let Some(Ok(msg)) = receiver.next().await {
            if let Ok(text) = msg.to_str() {
                let trimmed = text.trim();
                let mut users_lock = users.write().await;
                if trimmed != "SERVER" && !trimmed.is_empty() && !users_lock.contains_key(trimmed) {
                    username = trimmed.to_string();
                    users_lock.insert(username.clone(), sender.clone());
                    break;
                } else {
                    sender.lock().await.send(WsMessage::text(r#"["SERVER", "Username Error (RETRY)"]"#)).await.ok();
                }
            }
        } else {
            println!("Client disconnected before entering username.");
            return;  
        }
    }

    sender.lock().await.send(WsMessage::text(format!(r#"["SERVER", "Welcome, {}! Type :q to leave."]"#, username))).await.ok();

    // Send chat history
    {
        let history = message_history.lock().await;
        for msg in history.iter() {
            let json_msg = format!(r#"{}"#, msg); // Already formatted as JSON
            sender.lock().await.send(WsMessage::text(json_msg)).await.ok();
        }
    }

    let cloned_username = username.clone();
    let sender_clone = sender.clone();
    let users_clone = users.clone();
    
    // Background task to receive messages
    tokio::spawn(async move {
        while let Ok((msg, sender_name)) = rx.recv().await {
            let users_lock = users_clone.read().await;
            if sender_name != cloned_username && users_lock.contains_key(&cloned_username) {
                sender_clone.lock().await.send(WsMessage::text(msg)).await.ok();
            }
        }
    });

    let history_clone = message_history.clone();

    // Main chat loop
    while let Some(Ok(msg)) = receiver.next().await {
        if let Ok(text) = msg.to_str() {
            if text == ":q" {
                sender.lock().await.send(WsMessage::text(r#"["SERVER", "You have disconnected successfully."]"#)).await.ok();
                users.write().await.remove(&username);
                break;
            }
            if text == "/users" {
                let user_list = users.read().await.keys().cloned().collect::<Vec<String>>().join(", ");
                let users_message = format!(r#"["SERVER", "Online users: {}"]"#, user_list);
                sender.lock().await.send(WsMessage::text(users_message)).await.ok();
                continue;
            }

            // Store new message in history
            {
                let mut history = history_clone.lock().await;
                if history.len() >= 10 {
                    history.pop_front(); // Remove oldest message if over limit
                }
                history.push_back(format!(r#"["{}", "{}"]"#, username, text));
            }

            // Send the message to all users
            let message_json = format!(r#"["{}", "{}"]"#, username, text);
            tx.send((message_json, username.clone())).ok();
        }
    }

    // Disconnect logs
    println!("{} disconnected", username); 
    users.write().await.remove(&username);
    let disconnect_message = format!(r#"["SERVER", "{} has left the chat."]"#, username);
    tx.send((disconnect_message.to_string(), "SERVER".to_string())).ok();
}
