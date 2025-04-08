use std::collections::{HashMap, VecDeque};
use std::env;
use std::sync::Arc;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{broadcast, Mutex, RwLock};
use warp::ws::{Message as WsMessage, WebSocket};
use warp::{Reply, http::header::{SET_COOKIE, HeaderValue}};
use warp::cors::Cors;
use warp::Filter;
use serde::{Serialize, Deserialize};
use serde_json::json;
use dotenvy::dotenv;
use sqlx::PgPool;
use regex::Regex;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    let cors = warp::cors()
        .allow_any_origin()
        .allow_credentials(true)
        .allow_headers(vec!["Content-Type"]);

    dotenv().ok();

    let (tx, _rx) = broadcast::channel::<(String, String)>(10);
    let users = Arc::new(RwLock::new(HashMap::new()));
    let message_history = Arc::new(Mutex::new(VecDeque::new()));
    let username_regex = Arc::new(Regex::new(r"^[a-zA-Z0-9_.-]{4,}$").unwrap());
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let pool = PgPool::connect(&db_url).await.expect("Failed to connect to DB");
    let db = Arc::new(pool);

    let register = {
        let db = db.clone();
        let username_regex = username_regex.clone();
        warp::path("register")
            .and(warp::post())
            .and(warp::body::form())
            .and_then(move |form: HashMap<String, String>| {
                let db = db.clone();
                let username_regex = username_regex.clone();
                async move {
                    let username = form.get("username").unwrap_or(&"".to_string()).clone();
                    let password = form.get("password").unwrap_or(&"".to_string()).clone();

                    if !username_regex.is_match(&username) {
                        return Ok::<_, warp::Rejection>(warp::reply::html(
                            "Username must be minimum 4 characters, and can only contain letters, numbers, underscores (_), hyphens (-), or dots (.)"
                        ).into_response());
                    }
                    if password.len() < 8 || password.contains(' ') {
                        return Ok(warp::reply::html("Password must be at least 8 characters long and contain no spaces.").into_response());
                    }
                    let result = sqlx::query("INSERT INTO users (username, password) VALUES ($1, $2)")
                        .bind(&username)
                        .bind(&password)
                        .execute(&*db)
                        .await;

                    let reply = match result {
                        Ok(_) => warp::reply::html("Registered successfully!"),
                        Err(_) => warp::reply::html("Username already taken"),
                    };

                    Ok(reply.into_response())
                }
            })
    };

    let login = {
        let db = db.clone();
        warp::path("login")
            .and(warp::post())
            .and(warp::body::form())
            .and_then(move |form: HashMap<String, String>| {
                let db = db.clone();
                async move {
                    let username = form.get("username").unwrap_or(&"".to_string()).clone();
                    let password = form.get("password").unwrap_or(&"".to_string()).clone();
    
                    if username.is_empty() || password.is_empty() {
                        return Ok::<_, warp::Rejection>(
                            warp::reply::html("Missing username or password").into_response(),
                        );
                    }
    
                    let result = sqlx::query_scalar::<_, String>(
                        "SELECT password FROM users WHERE username = $1"
                    )
                    .bind(&username)
                    .fetch_optional(&*db)
                    .await;
    
                    let response = match result {
                        Ok(Some(db_password)) if db_password == password => {
                            let token = Uuid::new_v4().to_string();
                            sqlx::query("UPDATE users SET session_token = $1 WHERE username = $2")
                                .bind(&token)
                                .bind(&username)
                                .execute(&*db)
                                .await
                                .unwrap();
                            let cookie = format!("session={}; Path=/; HttpOnly; SameSite=None; Secure; Partitioned", token);
                            //let cookie = format!("session={}; Path=/; ", token);
                            let mut response = warp::reply::html("Login successful!").into_response();
                            response.headers_mut().insert(
                                SET_COOKIE,
                                HeaderValue::from_str(&cookie).unwrap(),
                            );
                            response
                        }
                        Ok(_) => warp::reply::html("Incorrect password").into_response(),
                        Err(_) => warp::reply::html("Username not found").into_response(),
                    };
    
                    Ok::<_, warp::Rejection>(response)
                }
            })
    };    

    let auth_check = {
        let db = db.clone();
        warp::path("auth-check")
            .and(warp::cookie::optional("session"))
            .and_then(move |session: Option<String>| {
                let db = db.clone();
                async move {
                    if let Some(token) = session {
                        let result = sqlx::query_scalar::<_, String>(
                            "SELECT username FROM users WHERE session_token = $1"
                        )
                        .bind(&token)
                        .fetch_optional(&*db)
                        .await;

                        if let Ok(Some(username)) = result {
                            let reply = warp::reply::json(&json!({
                                "status": "ok",
                                "username": username
                            }));

                            return Ok::<_, warp::Rejection>(reply.into_response());
                        }
                    }
                    let reply = warp::reply::json(&json!({
                        "status": "unauthorized"
                    }));
                    let response = warp::reply::with_status(reply, warp::http::StatusCode::UNAUTHORIZED).into_response();
                    Ok::<_, warp::Rejection>(response)
                }
            })
    };

    let ping = warp::path("ping").map(|| warp::reply::json(&"pong"));

    let chat = {
        let tx = tx.clone();
        let users = users.clone();
        let history = message_history.clone();
        let db = db.clone();

        warp::path("chat")
            .and(warp::cookie::optional("session"))
            .and(warp::ws())
            .and_then(move |session: Option<String>, ws: warp::ws::Ws| {
                let db = db.clone();
                let tx = tx.clone();
                let users = users.clone();
                let history = history.clone();

                async move {
                    if let Some(token) = session {
                        match sqlx::query_scalar::<_, String>(
                            "SELECT username FROM users WHERE session_token = $1"
                        )
                        .bind(&token)
                        .fetch_one(&*db)
                        .await
                        {
                            Ok(username) => {
                                let reply = ws.on_upgrade(move |socket| {
                                    handle_socket(socket, tx, users, history, username)
                                });
                                Ok::<_, warp::Rejection>(reply.into_response())
                            }
                            Err(_) => {
                                let resp = warp::reply::with_status(
                                    "Unauthorized",
                                    warp::http::StatusCode::UNAUTHORIZED,
                                );
                                Ok::<_, warp::Rejection>(resp.into_response())
                            }
                        }
                    } else {
                        let resp = warp::reply::with_status(
                            "Unauthorized",
                            warp::http::StatusCode::UNAUTHORIZED,
                        );
                        Ok::<_, warp::Rejection>(resp.into_response())
                    }
                }
            })
    };

    let routes = register
        .or(login)
        .or(ping)
        .or(chat)
        .or(auth_check)
        .with(cors);

    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string()).parse().expect("Invalid PORT value");
    println!("WebSocket server running on ws://0.0.0.0:{}", port);
    warp::serve(routes).run(([0, 0, 0, 0], port)).await;
}
    

async fn handle_socket(
    ws: WebSocket,
    tx: broadcast::Sender<(String, String)>,
    users: Arc<RwLock<HashMap<String, Arc<Mutex<futures_util::stream::SplitSink<WebSocket, WsMessage>>>>>>,
    message_history: Arc<Mutex<VecDeque<String>>>,
    username: String,
) {
    let (sender, mut receiver) = ws.split();
    let sender = Arc::new(Mutex::new(sender));
    let mut rx = tx.subscribe();

    #[derive(Deserialize, Serialize, Debug)]
    #[serde(tag = "type")]
    pub enum ClientMessage {
        #[serde(rename = "dm")]
        DirectMessage {
            to: String,
            msg: String,
        },
        #[serde(rename = "broadcast")]
        BroadcastMessage {
            msg: String,
        },
    }

    // Send message history
    {
        let history = message_history.lock().await;
        for msg in history.iter() {
            sender.lock().await.send(WsMessage::text(msg.clone())).await.ok();
        }
    }

    users.write().await.insert(username.clone(), sender.clone());

    let connection_message = json!(["SERVER", format!("{} has entered the chat.", username)]).to_string();
    tx.send((connection_message, "SERVER".to_string())).ok();

    let cloned_username = username.clone();
    let sender_clone = sender.clone();
    let users_clone = users.clone();

    // Spawn a task to handle broadcast messages for this user
    tokio::spawn(async move {
        while let Ok((msg, sender_name)) = rx.recv().await {
            let users_lock = users_clone.read().await;
            if  users_lock.contains_key(&cloned_username) {
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
                Ok(ClientMessage::DirectMessage { to, msg }) => {
                    let dm_text = json!([format!("Whisper::{}", username), msg]).to_string();
                    let users_map = users.read().await;
    
                    if let Some(recipient_sender) = users_map.get(&to) {
                        recipient_sender.lock().await.send(WsMessage::text(dm_text.clone())).await.ok();
                    }
    
                    // Echo back to sender
                    let echo_text = json!([username, format!("Whisper::{}\n{}", to, msg)]).to_string();
                    sender.lock().await.send(WsMessage::text(echo_text)).await.ok();
                }
    
                Ok(ClientMessage::BroadcastMessage { msg }) => {
                    match msg.as_str() {
                        "/users" => {
                            let user_list = users.read().await
                                .keys()
                                .cloned()
                                .collect::<Vec<_>>()
                                .join(", ");
                            let user_msg = json!(["SERVER", format!("Online users: {}", user_list)]).to_string();
                            sender.lock().await.send(WsMessage::text(user_msg)).await.ok();
                        }
    
                        ":q" => {
                            let exit_msg = json!(["SERVER", "You have disconnected successfully."]).to_string();
                            sender.lock().await.send(WsMessage::text(exit_msg)).await.ok();
                            users.write().await.remove(&username);
                            sender.lock().await.close().await.ok();
                            break;
                        }
    
                        _ => {
                            let json_msg = json!([username, msg]).to_string();
    
                            {
                                let mut history = history_clone.lock().await;
                                if history.len() >= 30 {
                                    history.pop_front();
                                }
                                history.push_back(json_msg.clone());
                            }
    
                            tx.send((json_msg, username.clone())).ok();
                        }
                    }
                }
    
                Err(_) => {
                    let warn_msg = json!(["SERVER", "Invalid message format."]).to_string();
                    sender.lock().await.send(WsMessage::text(warn_msg)).await.ok();
                }
            }
        }
    }
    
    println!("{} disconnected", username);
    users.write().await.remove(&username);
    let disconnect_message = json!(["SERVER", format!("{} has left the chat.", username)]).to_string();
    tx.send((disconnect_message, "SERVER".to_string())).ok();
}

