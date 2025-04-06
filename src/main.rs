use std::collections::{HashMap, VecDeque};
use std::env;
use std::sync::Arc;
use futures_util::{SinkExt, StreamExt};
use rusqlite::{params, Connection, Result};
use tokio::sync::{broadcast, Mutex, RwLock};
use warp::ws::{Message as WsMessage, WebSocket};
use warp::{Reply, http::header::{SET_COOKIE, HeaderValue}};
use warp::Filter;
use uuid::Uuid;
use warp::cors::Cors;
use serde_json::json;

fn init_db() -> Result<Connection> {
    let conn = Connection::open("users.db")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            password TEXT NOT NULL,
            session_token TEXT
        )",
        [],
    )?;
    Ok(conn)
}

#[tokio::main]
async fn main() {
    let cors = warp::cors()
    .allow_any_origin()
    .allow_credentials(true)
    .allow_headers(vec!["Content-Type"]);

    let (tx, _rx) = broadcast::channel::<(String, String)>(10);
    let users = Arc::new(RwLock::new(HashMap::new()));
    let message_history = Arc::new(Mutex::new(VecDeque::new()));
    let db = Arc::new(Mutex::new(init_db().expect("Failed to connect to DB")));

    let register = {
        let db = db.clone();
        warp::path("register")
            .and(warp::post())
            .and(warp::body::form())
            .and_then(move |form: HashMap<String, String>| {
                let db = db.clone();
                async move {
                    let username = form.get("username").unwrap_or(&"".to_string()).clone();
                    let password = form.get("password").unwrap_or(&"".to_string()).clone();
    
                    if username.is_empty() || password.is_empty() {
                        return Ok::<_, warp::Rejection>(warp::reply::html("Missing username or password"));
                    }
    
                    let conn = db.lock().await;
                    let result = conn.execute(
                        "INSERT INTO users (username, password) VALUES (?1, ?2)",
                        params![username, password],
                    );
    
                    let reply = match result {
                        Ok(_) => warp::reply::html("Registered successfully!"),
                        Err(_) => warp::reply::html("Username already taken"),
                    };
    
                    Ok(reply)
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
    
                    let conn = db.lock().await;
                    let mut stmt = conn.prepare("SELECT password FROM users WHERE username = ?1").unwrap();
                    let result: Result<String> = stmt.query_row(params![username], |row| row.get(0));
    
                    let response = match result {
                        Ok(db_password) if db_password == password => {
                            let token = Uuid::new_v4().to_string();
                            conn.execute(
                                "UPDATE users SET session_token = ?1 WHERE username = ?2",
                                params![token, username],
                            ).unwrap();
                            let cookie = format!("session={}; Path=/; SameSite=None; Secure;", token);
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
                        let conn = db.lock().await;
                        let user = conn.query_row(
                            "SELECT username FROM users WHERE session_token = ?1",
                            [token],
                            |row| row.get::<_, String>(0),
                        );
                        
                        if let Ok(username) = user {
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
            match session {
                Some(token) => {
                    let conn = db.lock().await;
                    let mut stmt = conn.prepare("SELECT username FROM users WHERE session_token = ?1").unwrap();
                    let user: Result<String> = stmt.query_row(params![token], |row| row.get(0));
    
                    match user {
                        Ok(username) => {
                            let reply = ws.on_upgrade(move |socket| {
                                handle_socket(socket, tx, users, history, username)
                            });
                            Ok::<_, warp::Rejection>(reply.into_response())
                        }
                        Err(_) => {
                            let redirect = warp::redirect::temporary(warp::http::Uri::from_static("/login"));
                            Ok::<_, warp::Rejection>(redirect.into_response())
                        }
                    }
                }
                None => {
                    let redirect = warp::redirect::temporary(warp::http::Uri::from_static("/login"));
                    Ok::<_, warp::Rejection>(redirect.into_response())
                }
            }
        }
    })
    };
    
    //let static_files = warp::fs::dir("./static");
    let routes = register.or(login).or(ping).or(chat).or(auth_check).with(cors);

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

    //sender.lock().await.send(WsMessage::text(format!(r#"["SERVER", "Welcome, {}! Type :q to leave."]"#, username))).await.ok();
    // Send chat history
    {
        let history = message_history.lock().await;
        for msg in history.iter() {
            let json_msg = format!(r#"{}"#, msg); // Already formatted as JSON
            sender.lock().await.send(WsMessage::text(json_msg)).await.ok();
        }
    }
    users.write().await.insert(username.clone(), sender.clone());
    let connection_message = format!(r#"["SERVER", "{} has entered the chat."]"#, username);
    tx.send((connection_message.to_string(), "SERVER".to_string())).ok();


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

    while let Some(Ok(msg)) = receiver.next().await {
        if let Ok(text) = msg.to_str() {
            if text.trim()==""{
                continue;
            }
            if text == ":q" {
                sender.lock().await.send(WsMessage::text(r#"["SERVER", "You have disconnected successfully."]"#)).await.ok();
                users.write().await.remove(&username);
                sender.lock().await.close().await.ok(); 
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
                if history.len() >= 30 {
                    history.pop_front(); // Remove oldest message if over limit
                }
                history.push_back(format!(r#"["{}", "{}"]"#, username, text));
            }

            // Send the message to all users
            let message_json = format!(r#"["{}", "{}"]"#, username, text);
            tx.send((message_json, username.clone())).ok();
        }
    }

    println!("{} disconnected", username); 
    users.write().await.remove(&username);
    let disconnect_message = format!(r#"["SERVER", "{} has left the chat."]"#, username);
    tx.send((disconnect_message.to_string(), "SERVER".to_string())).ok();
}
