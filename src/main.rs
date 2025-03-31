use std::collections::{HashMap, VecDeque};
use std::env;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, RwLock};
use warp::ws::{WebSocket, Message as WsMessage};
use warp::Filter;
use futures_util::{StreamExt, SinkExt};

#[tokio::main]
async fn main() {
    let (tx, _rx) = broadcast::channel::<(String, String)>(10);
    let users = Arc::new(RwLock::new(HashMap::new()));

    let message_history = Arc::new(Mutex::new(VecDeque::new())); 

    let chat = warp::path("chat")
        .and(warp::ws())
        .map(move |ws: warp::ws::Ws| {
            let tx = tx.clone();
            let users = users.clone();
            let history = message_history.clone();  
            ws.on_upgrade(move |socket| handle_socket(socket, tx, users, history))
        });

    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string()).parse().expect("Invalid PORT value");
    println!("WebSocket server running on ws://0.0.0.0:{}", port);

    warp::serve(chat).run(([0, 0, 0, 0], port)).await;
}

fn server_formatter(message: &str) -> String {
    format!("\n  [[SERVER]]  {}\n",message)
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
    sender.lock().await.send(WsMessage::text(server_formatter("Enter your username"))).await.ok();
    loop {    
        if let Some(Ok(msg)) = receiver.next().await {
            if let Ok(text) = msg.to_str() {
                let trimmed = text.trim();
                let mut users_lock = users.write().await;
                if !trimmed.is_empty() && !users_lock.contains_key(trimmed) {
                    username = trimmed.to_string();
                    users_lock.insert(username.clone(), sender.clone());
                    break;
                } else {
                    sender.lock().await.send(WsMessage::text(server_formatter("Username Error (RETRY)"))).await.ok();
                }
            }
        }
         else {
        // 🔹 Handle disconnect before entering username
        println!("Client disconnected before entering username.");
        return;  // Exit the function early
    }
    }
    sender.lock().await.send(WsMessage::text(server_formatter(&format!("Welcome, {}! Type :q to leave.", username)))).await.ok();

    // Send chat history
    {
        let history = message_history.lock().await;
        for msg in history.iter() {
            sender.lock().await.send(WsMessage::text(msg.clone())).await.ok();
        }
    }


    let cloned_username = username.clone();
    let sender_clone = sender.clone();

    let users_clone = users.clone();
    tokio::spawn(async move {
        while let Ok((msg, sender_name)) = rx.recv().await {
            let users_lock = users_clone.read().await;
            if  users_lock.contains_key(&cloned_username) {
                sender_clone.lock().await.send(WsMessage::text(msg)).await.ok();
            }
        }
    });

    let history_clone = message_history.clone(); // clone history before async move

    // Main chat loop
    while let Some(Ok(msg)) = receiver.next().await {
        if let Ok(text) = msg.to_str() {
            if text == ":q" {
                sender.lock().await.send(WsMessage::text(server_formatter("You have disconnected successfully."))).await.ok();
                users.write().await.remove(&username);
                break;
            }
            if text == "/users" {
                let user_list = users.read().await.keys().cloned().collect::<Vec<String>>().join(", ");
                sender.lock().await.send(WsMessage::text(server_formatter(&format!("Online users: {}", user_list)))).await.ok();
                continue;
            }

            // Store new message in history
            {
                let mut history = history_clone.lock().await;
                if history.len() >= 10 {
                    history.pop_front(); // Remove oldest message if over limit
                }
                history.push_back(format!("{}: {}", username, text));
            }

            tx.send((format!("{}: {}", username, text), username.clone())).ok();
        }
    }

    // 🔹 Handle unexpected disconnect
    println!("{} disconnected", username); 
    users.write().await.remove(&username);
    tx.send((server_formatter(&format!("{} has left the chat.", username)), "SERVER".to_string())).ok();
}

