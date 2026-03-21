mod character;
mod chat;
mod initiative;
mod inventory;
mod maps;
mod tokens;

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

pub async fn ws_upgrade(ws: WebSocketUpgrade, Query(query): Query<WsQuery>) -> impl IntoResponse {
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

fn is_gm(session_id: i32, user_id: i32) -> bool {
    use crate::db;
    use crate::schema::sessions;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();
    sessions::table
        .find(session_id)
        .select(sessions::gm_user_id)
        .first::<i32>(conn)
        .map(|gm_id| gm_id == user_id)
        .unwrap_or(false)
}

async fn handle_socket(socket: WebSocket, user_id: i32, username: String) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<ServerMessage>();

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

                let snapshot = maps::build_snapshot(session_id, user_id);
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
                    let chat_msg = chat::save_chat_message(
                        session_id, user_id, &username, &message, false, None,
                    );
                    SESSION_MANAGER.broadcast(
                        session_id,
                        &ServerMessage::ChatBroadcast { message: chat_msg },
                        None,
                    );
                }
            }

            ClientMessage::RollDice { expression } => {
                if let Some(session_id) = current_session {
                    match chat::parse_and_roll(&expression) {
                        Ok((rolls, total)) => {
                            let rolls_str = rolls
                                .iter()
                                .map(|r| r.to_string())
                                .collect::<Vec<_>>()
                                .join(", ");
                            let dice_json =
                                format!("{{\"rolls\":[{rolls_str}],\"total\":{total}}}");
                            let _chat_msg = chat::save_chat_message(
                                session_id,
                                user_id,
                                &username,
                                &format!("rolled {expression}: [{rolls_str}] = {total}"),
                                true,
                                Some(&dice_json),
                            );
                            SESSION_MANAGER.broadcast(
                                session_id,
                                &ServerMessage::DiceResult {
                                    username: username.clone(),
                                    expression,
                                    rolls,
                                    total,
                                },
                                None,
                            );
                        }
                        Err(e) => {
                            let _ = tx.send(ServerMessage::Error { message: e });
                        }
                    }
                }
            }

            ClientMessage::MoveToken { token_id, x, y } => {
                if let Some(session_id) = current_session {
                    if let Some(mut session) = SESSION_MANAGER.sessions.get_mut(&session_id) {
                        session.token_positions.insert(token_id, (x, y));
                    }
                    tokio::spawn(async move {
                        tokens::persist_token_position(token_id, x, y);
                    });
                    SESSION_MANAGER.broadcast(
                        session_id,
                        &ServerMessage::TokenMoved { token_id, x, y },
                        Some(&username),
                    );
                }
            }

            ClientMessage::PlaceToken {
                label,
                x,
                y,
                color,
                size,
                character_id,
                creature_id,
                image_url,
            } => {
                if let Some(session_id) = current_session {
                    if !is_gm(session_id, user_id) {
                        let _ = tx.send(ServerMessage::Error {
                            message: "Only the GM can place tokens".into(),
                        });
                        continue;
                    }
                    // Reject duplicate character tokens on the same map
                    if let Some(cid) = character_id {
                        use crate::schema::{maps, tokens as tokens_table};
                        use diesel::prelude::*;
                        let conn = &mut crate::db::get_conn();
                        let map_id: Option<i32> = maps::table
                            .filter(maps::session_id.eq(session_id))
                            .order(maps::id.desc())
                            .select(maps::id)
                            .first(conn)
                            .optional()
                            .unwrap_or(None);
                        if let Some(mid) = map_id {
                            let exists = tokens_table::table
                                .filter(tokens_table::map_id.eq(mid))
                                .filter(tokens_table::character_id.eq(cid))
                                .count()
                                .get_result::<i64>(conn)
                                .unwrap_or(0)
                                > 0;
                            if exists {
                                let _ = tx.send(ServerMessage::Error {
                                    message: "Character already has a token on this map".into(),
                                });
                                continue;
                            }
                        }
                    }
                    match tokens::place_token(
                        session_id,
                        &label,
                        x,
                        y,
                        &color,
                        size,
                        character_id,
                        creature_id,
                        image_url.as_deref(),
                    ) {
                        Ok(token_info) => {
                            if let Some(mut session) = SESSION_MANAGER.sessions.get_mut(&session_id)
                            {
                                session
                                    .token_positions
                                    .insert(token_info.id, (token_info.x, token_info.y));
                            }
                            SESSION_MANAGER.broadcast(
                                session_id,
                                &ServerMessage::TokenPlaced { token: token_info },
                                None,
                            );
                        }
                        Err(e) => {
                            let _ = tx.send(ServerMessage::Error { message: e });
                        }
                    }
                }
            }

            ClientMessage::PlaceAllPlayerTokens { x, y } => {
                if let Some(session_id) = current_session {
                    if !is_gm(session_id, user_id) {
                        let _ = tx.send(ServerMessage::Error {
                            message: "Only the GM can place all player tokens".into(),
                        });
                        continue;
                    }
                    // Get all characters in this session
                    let chars = {
                        use crate::schema::characters;
                        use diesel::prelude::*;
                        let conn = &mut crate::db::get_conn();
                        characters::table
                            .filter(characters::session_id.eq(session_id))
                            .select(crate::models::db_models::Character::as_select())
                            .load::<crate::models::db_models::Character>(conn)
                            .unwrap_or_default()
                    };
                    // Check which characters already have tokens on the active map
                    let existing_char_ids: std::collections::HashSet<i32> = {
                        use crate::schema::{maps, tokens};
                        use diesel::prelude::*;
                        let conn = &mut crate::db::get_conn();
                        let map_id: Option<i32> = maps::table
                            .filter(maps::session_id.eq(session_id))
                            .order(maps::id.desc())
                            .select(maps::id)
                            .first(conn)
                            .optional()
                            .unwrap_or(None);
                        if let Some(mid) = map_id {
                            tokens::table
                                .filter(tokens::map_id.eq(mid))
                                .filter(tokens::character_id.is_not_null())
                                .select(tokens::character_id)
                                .load::<Option<i32>>(conn)
                                .unwrap_or_default()
                                .into_iter()
                                .flatten()
                                .collect()
                        } else {
                            std::collections::HashSet::new()
                        }
                    };
                    // Place tokens in a rectangular grid centered at (x, y)
                    let to_place: Vec<_> = chars
                        .iter()
                        .filter(|ch| !existing_char_ids.contains(&ch.id))
                        .collect();
                    if to_place.is_empty() {
                        continue;
                    }
                    let n = to_place.len() as i32;
                    let cols = (n as f32).sqrt().ceil() as i32;
                    let rows = if cols > 0 { (n + cols - 1) / cols } else { 0 };
                    // Token size = 1 for all player characters (standard)
                    let cell = 1.0_f32;
                    let grid_w = cols as f32 * cell;
                    let grid_h = rows as f32 * cell;
                    let origin_x = (x - grid_w / 2.0).floor();
                    let origin_y = (y - grid_h / 2.0).floor();
                    for (i, ch) in to_place.iter().enumerate() {
                        let col = i as i32 % cols;
                        let row = i as i32 / cols;
                        let tok_x = origin_x + col as f32 * cell;
                        let tok_y = origin_y + row as f32 * cell;
                        match tokens::place_token(
                            session_id,
                            &ch.name,
                            tok_x,
                            tok_y,
                            "#4488cc",
                            1,
                            Some(ch.id),
                            None,
                            ch.portrait_url.as_deref(),
                        ) {
                            Ok(token_info) => {
                                if let Some(mut session) =
                                    SESSION_MANAGER.sessions.get_mut(&session_id)
                                {
                                    session
                                        .token_positions
                                        .insert(token_info.id, (tok_x, tok_y));
                                }
                                SESSION_MANAGER.broadcast(
                                    session_id,
                                    &ServerMessage::TokenPlaced { token: token_info },
                                    None,
                                );
                            }
                            Err(e) => {
                                let _ = tx.send(ServerMessage::Error { message: e });
                            }
                        }
                    }
                }
            }

            ClientMessage::RemoveToken { token_id } => {
                if let Some(session_id) = current_session {
                    if !is_gm(session_id, user_id) {
                        let _ = tx.send(ServerMessage::Error {
                            message: "Only the GM can remove tokens".into(),
                        });
                        continue;
                    }
                    tokens::remove_token(token_id);
                    if let Some(mut session) = SESSION_MANAGER.sessions.get_mut(&session_id) {
                        session.token_positions.remove(&token_id);
                    }
                    SESSION_MANAGER.broadcast(
                        session_id,
                        &ServerMessage::TokenRemoved { token_id },
                        None,
                    );
                }
            }

            ClientMessage::RevealFog { cells } => {
                if let Some(session_id) = current_session {
                    if !is_gm(session_id, user_id) {
                        let _ = tx.send(ServerMessage::Error {
                            message: "Only the GM can modify fog".into(),
                        });
                        continue;
                    }
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
                    if !is_gm(session_id, user_id) {
                        let _ = tx.send(ServerMessage::Error {
                            message: "Only the GM can modify fog".into(),
                        });
                        continue;
                    }
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

            ClientMessage::SetMap { map_id } => {
                if let Some(session_id) = current_session {
                    if !is_gm(session_id, user_id) {
                        let _ = tx.send(ServerMessage::Error {
                            message: "Only the GM can change the map".into(),
                        });
                        continue;
                    }
                    match maps::load_map_with_tokens(map_id) {
                        Ok((map_info, token_list, fog_cells)) => {
                            if let Some(mut session) = SESSION_MANAGER.sessions.get_mut(&session_id)
                            {
                                session.active_map_id = Some(map_id);
                                session.token_positions.clear();
                                for t in &token_list {
                                    session.token_positions.insert(t.id, (t.x, t.y));
                                }
                                session.revealed_fog.clear();
                                for cell in &fog_cells {
                                    session.revealed_fog.insert(*cell);
                                }
                            }
                            SESSION_MANAGER.broadcast(
                                session_id,
                                &ServerMessage::MapChanged {
                                    map: map_info,
                                    tokens: token_list,
                                    fog: fog_cells,
                                },
                                None,
                            );
                        }
                        Err(e) => {
                            let _ = tx.send(ServerMessage::Error { message: e });
                        }
                    }
                }
            }

            ClientMessage::UpdateInitiative { entries } => {
                if let Some(session_id) = current_session {
                    if !is_gm(session_id, user_id) {
                        let _ = tx.send(ServerMessage::Error {
                            message: "Only the GM can update initiative".into(),
                        });
                        continue;
                    }
                    initiative::save_initiative(session_id, &entries);
                    if let Some(mut session) = SESSION_MANAGER.sessions.get_mut(&session_id) {
                        session.initiative_order = entries.clone();
                    }
                    SESSION_MANAGER.broadcast(
                        session_id,
                        &ServerMessage::InitiativeUpdated { entries },
                        None,
                    );
                }
            }

            ClientMessage::RollCharacterInitiative { character_id } => {
                if let Some(session_id) = current_session {
                    // Check if initiative is locked
                    let locked = SESSION_MANAGER
                        .sessions
                        .get(&session_id)
                        .map(|s| s.initiative_locked)
                        .unwrap_or(false);
                    if locked {
                        let _ = tx.send(ServerMessage::Error {
                            message: "Initiative is locked".into(),
                        });
                        continue;
                    }
                    match initiative::roll_character_initiative(session_id, character_id) {
                        Ok(result) => {
                            initiative::broadcast_initiative_roll(
                                session_id, user_id, &username, result,
                            );
                        }
                        Err(e) => {
                            let _ = tx.send(ServerMessage::Error { message: e });
                        }
                    }
                }
            }

            ClientMessage::RollCreatureInitiative { creature_id, label } => {
                if let Some(session_id) = current_session {
                    match initiative::roll_creature_initiative(session_id, creature_id, &label) {
                        Ok(result) => {
                            initiative::broadcast_initiative_roll(
                                session_id, user_id, &username, result,
                            );
                        }
                        Err(e) => {
                            let _ = tx.send(ServerMessage::Error { message: e });
                        }
                    }
                }
            }

            ClientMessage::SetInitiativeLock { locked } => {
                if let Some(session_id) = current_session {
                    if !is_gm(session_id, user_id) {
                        let _ = tx.send(ServerMessage::Error {
                            message: "Only the GM can lock/unlock initiative".into(),
                        });
                        continue;
                    }
                    if let Some(mut session) = SESSION_MANAGER.sessions.get_mut(&session_id) {
                        session.initiative_locked = locked;
                    }
                    SESSION_MANAGER.broadcast(
                        session_id,
                        &ServerMessage::InitiativeLockChanged { locked },
                        None,
                    );
                }
            }

            ClientMessage::SetMapBackground { background_url } => {
                if let Some(session_id) = current_session {
                    if !is_gm(session_id, user_id) {
                        let _ = tx.send(ServerMessage::Error {
                            message: "Only the GM can change the map background".into(),
                        });
                        continue;
                    }
                    let map_id = SESSION_MANAGER
                        .sessions
                        .get(&session_id)
                        .and_then(|s| s.active_map_id);
                    if let Some(map_id) = map_id {
                        use crate::schema::maps;
                        use diesel::prelude::*;
                        let conn = &mut crate::db::get_conn();
                        let _ = diesel::update(maps::table.find(map_id))
                            .set(maps::background_url.eq(&background_url))
                            .execute(conn);
                        SESSION_MANAGER.broadcast(
                            session_id,
                            &ServerMessage::MapBackgroundChanged { background_url },
                            None,
                        );
                    } else {
                        let _ = tx.send(ServerMessage::Error {
                            message: "No active map".into(),
                        });
                    }
                }
            }

            ClientMessage::SetMapDefaultColor { color } => {
                if let Some(session_id) = current_session {
                    if !is_gm(session_id, user_id) {
                        let _ = tx.send(ServerMessage::Error {
                            message: "Only the GM can change the default token color".into(),
                        });
                        continue;
                    }
                    let map_id = SESSION_MANAGER
                        .sessions
                        .get(&session_id)
                        .and_then(|s| s.active_map_id);
                    if let Some(map_id) = map_id {
                        use crate::schema::maps;
                        use diesel::prelude::*;
                        let conn = &mut crate::db::get_conn();
                        let _ = diesel::update(maps::table.find(map_id))
                            .set(maps::default_token_color.eq(&color))
                            .execute(conn);
                        SESSION_MANAGER.broadcast(
                            session_id,
                            &ServerMessage::MapDefaultColorChanged {
                                default_token_color: color,
                            },
                            None,
                        );
                    } else {
                        let _ = tx.send(ServerMessage::Error {
                            message: "No active map".into(),
                        });
                    }
                }
            }

            ClientMessage::UpdateCharacterField {
                character_id,
                field_path,
                value,
            } => {
                if let Some(session_id) = current_session {
                    if let Err(e) = character::update_character_field(
                        character_id,
                        user_id,
                        &field_path,
                        &value,
                    ) {
                        let _ = tx.send(ServerMessage::Error { message: e });
                        continue;
                    }
                    SESSION_MANAGER.broadcast(
                        session_id,
                        &ServerMessage::CharacterUpdated {
                            character_id,
                            field_path,
                            value,
                        },
                        None,
                    );
                }
            }

            ClientMessage::AddInventoryItem {
                name,
                description,
                quantity,
                is_party_item,
            } => {
                if let Some(session_id) = current_session {
                    match inventory::add_inventory_item(
                        session_id,
                        &name,
                        &description,
                        quantity,
                        is_party_item,
                    ) {
                        Ok(items) => {
                            SESSION_MANAGER.broadcast(
                                session_id,
                                &ServerMessage::InventoryUpdated { items },
                                None,
                            );
                        }
                        Err(e) => {
                            let _ = tx.send(ServerMessage::Error { message: e });
                        }
                    }
                }
            }

            ClientMessage::RemoveInventoryItem { item_id } => {
                if let Some(session_id) = current_session {
                    inventory::remove_inventory_item(item_id);
                    let items = inventory::load_inventory(session_id);
                    SESSION_MANAGER.broadcast(
                        session_id,
                        &ServerMessage::InventoryUpdated { items },
                        None,
                    );
                }
            }

            ClientMessage::UpdateInventoryItem {
                item_id,
                name,
                description,
                quantity,
            } => {
                if let Some(session_id) = current_session {
                    inventory::update_inventory_item(
                        item_id,
                        name.as_deref(),
                        description.as_deref(),
                        quantity,
                    );
                    let items = inventory::load_inventory(session_id);
                    SESSION_MANAGER.broadcast(
                        session_id,
                        &ServerMessage::InventoryUpdated { items },
                        None,
                    );
                }
            }

            ClientMessage::MoveTokens { moves } => {
                if let Some(session_id) = current_session {
                    for &(token_id, x, y) in &moves {
                        if let Some(mut session) = SESSION_MANAGER.sessions.get_mut(&session_id) {
                            session.token_positions.insert(token_id, (x, y));
                        }
                        tokens::persist_token_position(token_id, x, y);
                    }
                    SESSION_MANAGER.broadcast(
                        session_id,
                        &ServerMessage::TokensMoved {
                            moves: moves.clone(),
                        },
                        Some(&username),
                    );
                }
            }

            ClientMessage::RotateTokens { rotations } => {
                if let Some(session_id) = current_session {
                    {
                        use crate::schema::tokens;
                        use diesel::prelude::*;
                        let conn = &mut crate::db::get_conn();
                        for &(token_id, rotation) in &rotations {
                            let _ = diesel::update(tokens::table.find(token_id))
                                .set(tokens::rotation.eq(rotation))
                                .execute(conn);
                        }
                    }
                    SESSION_MANAGER.broadcast(
                        session_id,
                        &ServerMessage::TokensRotated {
                            rotations: rotations.clone(),
                        },
                        None,
                    );
                }
            }

            ClientMessage::UpdateTokenConditions {
                token_id,
                conditions,
            } => {
                if let Some(session_id) = current_session {
                    {
                        use crate::schema::token_instances;
                        use diesel::prelude::*;
                        let conn = &mut crate::db::get_conn();
                        let json =
                            serde_json::to_string(&conditions).unwrap_or_else(|_| "[]".into());
                        let _ = diesel::update(
                            token_instances::table.filter(token_instances::token_id.eq(token_id)),
                        )
                        .set(token_instances::conditions_json.eq(&json))
                        .execute(conn);
                    }
                    SESSION_MANAGER.broadcast(
                        session_id,
                        &ServerMessage::TokenConditionsUpdated {
                            token_id,
                            conditions,
                        },
                        None,
                    );
                }
            }

            ClientMessage::Ping { x, y } => {
                if let Some(session_id) = current_session {
                    // Look up user's ping color
                    let color = {
                        use crate::schema::users;
                        use diesel::prelude::*;
                        let conn = &mut crate::db::get_conn();
                        users::table
                            .find(user_id)
                            .select(users::ping_color)
                            .first::<String>(conn)
                            .unwrap_or_else(|_| "#ffcc00".to_string())
                    };
                    SESSION_MANAGER.broadcast(
                        session_id,
                        &ServerMessage::PingBroadcast {
                            username: username.clone(),
                            x,
                            y,
                            color,
                        },
                        None,
                    );
                }
            }

            ClientMessage::SyncViewport { x, y, zoom } => {
                if let Some(session_id) = current_session {
                    if !is_gm(session_id, user_id) {
                        let _ = tx.send(ServerMessage::Error {
                            message: "Only the GM can sync viewport".into(),
                        });
                        continue;
                    }
                    SESSION_MANAGER.broadcast(
                        session_id,
                        &ServerMessage::ViewportSynced { x, y, zoom },
                        Some(&username),
                    );
                }
            }

            ClientMessage::SetPingColor { color } => {
                // Validate color is a hex string
                if color.starts_with('#') && color.len() <= 9 {
                    use crate::schema::users;
                    use diesel::prelude::*;
                    let conn = &mut crate::db::get_conn();
                    let _ = diesel::update(users::table.find(user_id))
                        .set(users::ping_color.eq(&color))
                        .execute(conn);
                }
            }
            ClientMessage::SetSuppressTooltips { suppress } => {
                use crate::schema::users;
                use diesel::prelude::*;
                let conn = &mut crate::db::get_conn();
                let _ = diesel::update(users::table.find(user_id))
                    .set(users::suppress_tooltips.eq(if suppress { 1 } else { 0 }))
                    .execute(conn);
            }
        }
    }

    // Client disconnected
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
