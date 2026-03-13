use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Query, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::auth;
use crate::ws::messages::{ClientMessage, ServerMessage};
use crate::ws::session::SESSION_MANAGER;

#[derive(Deserialize)]
pub struct WsQuery {
    token: String,
}

pub async fn ws_upgrade(
    ws: WebSocketUpgrade,
    Query(query): Query<WsQuery>,
) -> impl IntoResponse {
    let claims = match auth::verify_jwt(&query.token) {
        Ok(c) => c,
        Err(_) => {
            return axum::http::StatusCode::UNAUTHORIZED.into_response();
        }
    };

    let (user_id, username) = match auth::parse_claims_sub(&claims.sub) {
        Some(parsed) => parsed,
        None => {
            return axum::http::StatusCode::UNAUTHORIZED.into_response();
        }
    };

    ws.on_upgrade(move |socket| handle_socket(socket, user_id, username))
        .into_response()
}

async fn handle_socket(socket: WebSocket, user_id: i32, username: String) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<ServerMessage>();

    // Spawn task to forward messages from channel to WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if ws_sender.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    let mut current_session: Option<i32> = None;
    let username_clone = username.clone();

    // Receive messages from WebSocket
    while let Some(Ok(msg)) = ws_receiver.next().await {
        let text = match msg {
            Message::Text(t) => t.to_string(),
            Message::Close(_) => break,
            _ => continue,
        };

        let client_msg: ClientMessage = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(e) => {
                let _ = tx.send(ServerMessage::Error {
                    message: format!("Invalid message: {e}"),
                });
                continue;
            }
        };

        match client_msg {
            ClientMessage::JoinSession { session_id } => {
                // Leave previous session if any
                if let Some(prev_id) = current_session.take() {
                    SESSION_MANAGER.remove_client(prev_id, &username);
                    SESSION_MANAGER.broadcast(
                        prev_id,
                        &ServerMessage::PlayerLeft {
                            username: username.clone(),
                        },
                        None,
                    );
                }

                SESSION_MANAGER.add_client(session_id, username.clone(), tx.clone());
                current_session = Some(session_id);

                // Build snapshot from database
                let snapshot = build_snapshot(session_id, &username).await;
                let _ = tx.send(ServerMessage::SessionJoined { snapshot });

                SESSION_MANAGER.broadcast(
                    session_id,
                    &ServerMessage::PlayerJoined {
                        username: username.clone(),
                    },
                    Some(&username),
                );
            }

            ClientMessage::LeaveSession => {
                if let Some(session_id) = current_session.take() {
                    SESSION_MANAGER.remove_client(session_id, &username);
                    SESSION_MANAGER.broadcast(
                        session_id,
                        &ServerMessage::PlayerLeft {
                            username: username.clone(),
                        },
                        None,
                    );
                }
            }

            ClientMessage::ChatMessage { message } => {
                if let Some(session_id) = current_session {
                    let chat_msg = save_chat_message(session_id, user_id, &username, &message);
                    SESSION_MANAGER.broadcast(
                        session_id,
                        &ServerMessage::ChatBroadcast { message: chat_msg },
                        None,
                    );
                }
            }

            ClientMessage::RollDice { expression } => {
                if let Some(session_id) = current_session {
                    match parse_and_roll(&expression) {
                        Ok((rolls, total)) => {
                            let chat_msg = save_chat_message(
                                session_id,
                                user_id,
                                &username,
                                &format!("rolled {expression}: {total}"),
                            );
                            // Store dice result in chat
                            SESSION_MANAGER.broadcast(
                                session_id,
                                &ServerMessage::DiceResult {
                                    username: username.clone(),
                                    expression: expression.clone(),
                                    rolls,
                                    total,
                                },
                                None,
                            );
                            let _ = chat_msg; // already broadcast
                        }
                        Err(e) => {
                            let _ = tx.send(ServerMessage::Error { message: e });
                        }
                    }
                }
            }

            ClientMessage::MoveToken { token_id, x, y } => {
                if let Some(session_id) = current_session {
                    // Update in-memory state
                    if let Some(mut session) = SESSION_MANAGER.sessions.get_mut(&session_id) {
                        session.token_positions.insert(token_id, (x, y));
                    }
                    // Persist to DB asynchronously
                    tokio::spawn(async move {
                        persist_token_position(token_id, x, y);
                    });
                    SESSION_MANAGER.broadcast(
                        session_id,
                        &ServerMessage::TokenMoved { token_id, x, y },
                        Some(&username),
                    );
                }
            }

            ClientMessage::RevealFog { cells } => {
                if let Some(session_id) = current_session {
                    // TODO: check GM role
                    if let Some(mut session) = SESSION_MANAGER.sessions.get_mut(&session_id) {
                        for cell in &cells {
                            session.revealed_fog.insert(*cell);
                        }
                    }
                    SESSION_MANAGER.broadcast(
                        session_id,
                        &ServerMessage::FogUpdated {
                            revealed: cells,
                            hidden: vec![],
                        },
                        None,
                    );
                }
            }

            ClientMessage::HideFog { cells } => {
                if let Some(session_id) = current_session {
                    if let Some(mut session) = SESSION_MANAGER.sessions.get_mut(&session_id) {
                        for cell in &cells {
                            session.revealed_fog.remove(cell);
                        }
                    }
                    SESSION_MANAGER.broadcast(
                        session_id,
                        &ServerMessage::FogUpdated {
                            revealed: vec![],
                            hidden: cells,
                        },
                        None,
                    );
                }
            }

            // Placeholder handlers for remaining message types
            _ => {
                let _ = tx.send(ServerMessage::Error {
                    message: "Not yet implemented".to_string(),
                });
            }
        }
    }

    // Client disconnected - clean up
    if let Some(session_id) = current_session {
        SESSION_MANAGER.remove_client(session_id, &username_clone);
        SESSION_MANAGER.broadcast(
            session_id,
            &ServerMessage::PlayerLeft {
                username: username_clone,
            },
            None,
        );
    }

    send_task.abort();
}

async fn build_snapshot(
    session_id: i32,
    _username: &str,
) -> crate::ws::messages::GameStateSnapshot {
    use crate::db;
    use crate::models::db_models::Session;
    use crate::schema::sessions;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    let session = sessions::table
        .find(session_id)
        .select(Session::as_select())
        .first(conn)
        .ok();

    let session_name = session.map(|s| s.name).unwrap_or_default();

    // Get connected players
    let players: Vec<String> = SESSION_MANAGER
        .sessions
        .get(&session_id)
        .map(|s| s.clients.iter().map(|e| e.key().clone()).collect())
        .unwrap_or_default();

    crate::ws::messages::GameStateSnapshot {
        session_id,
        session_name,
        players,
        map: None,
        tokens: vec![],
        fog: vec![],
        initiative: vec![],
        recent_chat: vec![],
        inventory: vec![],
    }
}

fn save_chat_message(
    session_id: i32,
    user_id: i32,
    username: &str,
    message: &str,
) -> crate::models::ChatMessageInfo {
    use crate::db;
    use crate::models::db_models::NewChatMessage;
    use crate::schema::chat_messages;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    let new_msg = NewChatMessage {
        session_id,
        user_id,
        message,
        is_dice_roll: false,
        dice_result: None,
    };

    let _ = diesel::insert_into(chat_messages::table)
        .values(&new_msg)
        .execute(conn);

    crate::models::ChatMessageInfo {
        id: 0,
        username: username.to_string(),
        message: message.to_string(),
        is_dice_roll: false,
        dice_result: None,
        created_at: String::new(),
    }
}

fn persist_token_position(token_id: i32, x: f32, y: f32) {
    use crate::db;
    use crate::schema::tokens;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();
    let _ = diesel::update(tokens::table.find(token_id))
        .set((tokens::x.eq(x), tokens::y.eq(y)))
        .execute(conn);
}

/// Parse dice expressions like "2d6+3", "1d20", "4d8-2"
fn parse_and_roll(expression: &str) -> Result<(Vec<i32>, i32), String> {
    use rand::Rng;

    let expr = expression.trim().to_lowercase();

    let (dice_part, modifier) = if let Some(pos) = expr.rfind('+') {
        let (d, m) = expr.split_at(pos);
        let modifier: i32 = m[1..]
            .parse()
            .map_err(|_| format!("Invalid modifier in '{expression}'"))?;
        (d, modifier)
    } else if let Some(pos) = expr.rfind('-') {
        if pos == 0 {
            return Err(format!("Invalid expression: '{expression}'"));
        }
        let (d, m) = expr.split_at(pos);
        let modifier: i32 = m
            .parse()
            .map_err(|_| format!("Invalid modifier in '{expression}'"))?;
        (d, modifier)
    } else {
        (expr.as_str(), 0)
    };

    let parts: Vec<&str> = dice_part.split('d').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid dice expression: '{expression}'"));
    }

    let count: i32 = if parts[0].is_empty() {
        1
    } else {
        parts[0]
            .parse()
            .map_err(|_| format!("Invalid dice count in '{expression}'"))?
    };

    let sides: i32 = parts[1]
        .parse()
        .map_err(|_| format!("Invalid dice sides in '{expression}'"))?;

    if count < 1 || count > 100 || sides < 1 || sides > 1000 {
        return Err("Dice values out of range".to_string());
    }

    let mut rng = rand::thread_rng();
    let rolls: Vec<i32> = (0..count).map(|_| rng.gen_range(1..=sides)).collect();
    let total: i32 = rolls.iter().sum::<i32>() + modifier;

    Ok((rolls, total))
}
