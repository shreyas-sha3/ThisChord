use warp::Filter;
use tokio::sync::broadcast;
use futures_util::{StreamExt, SinkExt};
use warp::ws::{WebSocket, Message as WsMessage};

#[tokio::main]
async fn main() {
    let (tx, _rx) = broadcast::channel::<(String, String)>(10); // Broadcast messages with usernames

    let chat = warp::path("chat")
        .and(warp::ws())
        .map(move |ws: warp::ws::Ws| {
            let tx = tx.clone();
            ws.on_upgrade(move |socket| handle_socket(socket, tx))
        });

    println!("WebSocket server running on ws://localhost:3030/chat");

    warp::serve(chat).run(([127, 0, 0, 1], 3030)).await;
}

/// **Handles the WebSocket connection**
async fn handle_socket(ws: WebSocket, tx: broadcast::Sender<(String, String)>) {
    let (mut sender, mut receiver) = ws.split();
    let mut rx = tx.subscribe();

    // 🔹 Ask for username
    sender.send(WsMessage::text("Enter your username:")).await.ok();
    let username = match receiver.next().await {
        Some(Ok(msg)) if msg.to_str().is_ok() => msg.to_str().unwrap().trim().to_string(),
        _ => "Guest".to_string(), // Default if no input
    };

    sender.send(WsMessage::text(format!("Welcome, {}! Start chatting.", username)))
        .await
        .ok();

    // 🔹 Background task to receive messages
    let cloned_username = username.clone();
    tokio::spawn(async move {
        while let Ok((msg, sender_name)) = rx.recv().await {
            if sender_name != cloned_username {
                sender.send(WsMessage::text(msg)).await.ok();
            }
        }
    });

    // 🔹 Main chat loop
    while let Some(Ok(msg)) = receiver.next().await {
        if let Ok(text) = msg.to_str() {
            if text == ":q" {
                tx.send((format!("{} disconnected.", username), username.clone())).ok();
                println!("{} disconnected", username);
                break;
            }
            tx.send((format!("{}: {}", username, text), username.clone())).ok();
        }
    }

    println!("Closing connection for {}", username);
}

