//use futures_util::{SinkExt, StreamExt};
//use warp::ws::Message as WsMessage;
//use serde::{Serialize, Deserialize};
use std::collections::{HashMap};
use std::env;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use warp::{Filter, Reply, http::header::{SET_COOKIE, HeaderValue}};
use serde_json::json;
use dotenvy::dotenv;
use sqlx::PgPool;
use regex::Regex;
use uuid::Uuid;

mod db;
mod socket; 
use crate::socket::handle_socket;
mod message;

#[tokio::main]
async fn main() {
    let cors = warp::cors()
    .allow_origins(vec![
        "http://127.0.0.1:5500",
        "http://localhost:5500",               
        "https://thischord.pages.dev",         
    ])
    .allow_any_origin()
    .allow_methods(vec!["GET", "POST"])
    .allow_headers(vec!["content-type"])
    .allow_credentials(true); 

    dotenv().ok();

    let (tx, _rx) = broadcast::channel::<(String, String)>(10);
    let users = Arc::new(RwLock::new(HashMap::new()));
    //let message_history = Arc::new(Mutex::new(VecDeque::new()));
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

    let chat = {
        let tx = tx.clone();
        let users = users.clone();
        //let history = message_history.clone();
        let db = db.clone();

        warp::path("chat")
            .and(warp::cookie::optional("session"))
            .and(warp::ws())
            .and_then(move |session: Option<String>, ws: warp::ws::Ws| {
                let db = db.clone();
                let tx = tx.clone();
                let users = users.clone();

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
                                    handle_socket(socket, tx, users, db, username)
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

    let ping = warp::path("ping").map(|| warp::reply::json(&"pong"));

    let routes = register
        .or(login)
        .or(ping)
        .or(auth_check)
        .or(chat)
        .with(cors);

    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string()).parse().expect("Invalid PORT value");
    println!("WebSocket server running on ws://0.0.0.0:{}", port);
    warp::serve(routes).run(([0, 0, 0, 0], port)).await;
}
    

