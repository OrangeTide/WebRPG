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

                let snapshot = build_snapshot(session_id);
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
                    let chat_msg =
                        save_chat_message(session_id, user_id, &username, &message, false, None);
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
                            let rolls_str = rolls
                                .iter()
                                .map(|r| r.to_string())
                                .collect::<Vec<_>>()
                                .join(", ");
                            let dice_json =
                                format!("{{\"rolls\":[{rolls_str}],\"total\":{total}}}");
                            let _chat_msg = save_chat_message(
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
                        persist_token_position(token_id, x, y);
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
                    match place_token(
                        session_id,
                        &label,
                        x,
                        y,
                        &color,
                        size,
                        creature_id,
                        image_url.as_deref(),
                    ) {
                        Ok(token_info) => {
                            if let Some(mut session) = SESSION_MANAGER.sessions.get_mut(&session_id)
                            {
                                session.token_positions.insert(token_info.id, (x, y));
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

            ClientMessage::RemoveToken { token_id } => {
                if let Some(session_id) = current_session {
                    if !is_gm(session_id, user_id) {
                        let _ = tx.send(ServerMessage::Error {
                            message: "Only the GM can remove tokens".into(),
                        });
                        continue;
                    }
                    remove_token(token_id);
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

            ClientMessage::UpdateTokenHp {
                token_id,
                hp_change,
            } => {
                if let Some(session_id) = current_session {
                    if !is_gm(session_id, user_id) {
                        let _ = tx.send(ServerMessage::Error {
                            message: "Only the GM can update token HP".into(),
                        });
                        continue;
                    }
                    match update_token_hp(token_id, hp_change) {
                        Ok((current_hp, max_hp)) => {
                            SESSION_MANAGER.broadcast(
                                session_id,
                                &ServerMessage::TokenHpUpdated {
                                    token_id,
                                    current_hp,
                                    max_hp,
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
                    match load_map_with_tokens(map_id) {
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
                    save_initiative(session_id, &entries);
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
                    match roll_character_initiative(session_id, character_id) {
                        Ok(result) => {
                            broadcast_initiative_roll(session_id, user_id, &username, result);
                        }
                        Err(e) => {
                            let _ = tx.send(ServerMessage::Error { message: e });
                        }
                    }
                }
            }

            ClientMessage::RollCreatureInitiative { creature_id, label } => {
                if let Some(session_id) = current_session {
                    match roll_creature_initiative(session_id, creature_id, &label) {
                        Ok(result) => {
                            broadcast_initiative_roll(session_id, user_id, &username, result);
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

            ClientMessage::UpdateCharacterField {
                character_id,
                field_path,
                value,
            } => {
                if let Some(session_id) = current_session {
                    if let Err(e) =
                        update_character_field(character_id, user_id, &field_path, &value)
                    {
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
                    match add_inventory_item(
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
                    remove_inventory_item(item_id);
                    let items = load_inventory(session_id);
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
                    update_inventory_item(
                        item_id,
                        name.as_deref(),
                        description.as_deref(),
                        quantity,
                    );
                    let items = load_inventory(session_id);
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
                        persist_token_position(token_id, x, y);
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

// ===== Helper functions =====

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

fn build_snapshot(session_id: i32) -> crate::ws::messages::GameStateSnapshot {
    use crate::db;
    use crate::models::db_models::*;
    use crate::schema::*;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    let session_name = sessions::table
        .find(session_id)
        .select(sessions::name)
        .first::<String>(conn)
        .unwrap_or_default();

    let players: Vec<String> = SESSION_MANAGER
        .sessions
        .get(&session_id)
        .map(|s| s.clients.iter().map(|e| e.key().clone()).collect())
        .unwrap_or_default();

    // Load active map (most recent map for this session)
    let map_row: Option<Map> = maps::table
        .filter(maps::session_id.eq(session_id))
        .order(maps::id.desc())
        .select(Map::as_select())
        .first(conn)
        .optional()
        .unwrap_or(None);

    let (map_info, token_list, fog_cells) = if let Some(m) = map_row {
        let map_id = m.id;

        // Ensure active_map_id is set so subsequent operations (e.g. SetMapBackground) work
        if let Some(mut session) = SESSION_MANAGER.sessions.get_mut(&session_id) {
            session.active_map_id = Some(map_id);
        }

        let map_info = crate::models::MapInfo {
            id: m.id,
            name: m.name,
            width: m.width,
            height: m.height,
            cell_size: m.cell_size,
            background_url: m.background_url,
        };

        let db_tokens: Vec<Token> = tokens::table
            .filter(tokens::map_id.eq(map_id))
            .select(Token::as_select())
            .load(conn)
            .unwrap_or_default();

        let token_list: Vec<crate::models::TokenInfo> = db_tokens
            .into_iter()
            .map(|t| {
                let instance: Option<TokenInstance> = token_instances::table
                    .filter(token_instances::token_id.eq(t.id))
                    .select(TokenInstance::as_select())
                    .first(conn)
                    .optional()
                    .unwrap_or(None);

                let conditions: Vec<String> = instance
                    .as_ref()
                    .and_then(|i| serde_json::from_str(&i.conditions_json).ok())
                    .unwrap_or_default();

                crate::models::TokenInfo {
                    id: t.id,
                    label: t.label,
                    x: t.x,
                    y: t.y,
                    color: t.color,
                    size: t.size,
                    visible: t.visible,
                    current_hp: instance.as_ref().map(|i| i.current_hp),
                    max_hp: instance.as_ref().map(|i| i.max_hp),
                    image_url: t.image_url,
                    rotation: t.rotation,
                    conditions,
                }
            })
            .collect();

        let fog_cells: Vec<(i32, i32)> = fog_of_war::table
            .filter(fog_of_war::map_id.eq(map_id))
            .select((fog_of_war::x, fog_of_war::y))
            .load(conn)
            .unwrap_or_default();

        (Some(map_info), token_list, fog_cells)
    } else {
        (None, vec![], vec![])
    };

    // Load initiative
    let init_entries: Vec<InitiativeEntry> = initiative::table
        .filter(initiative::session_id.eq(session_id))
        .order(initiative::sort_order.asc())
        .select(InitiativeEntry::as_select())
        .load(conn)
        .unwrap_or_default();

    let initiative_list: Vec<crate::models::InitiativeEntryInfo> = init_entries
        .into_iter()
        .map(|e| crate::models::InitiativeEntryInfo {
            id: e.id,
            label: e.label,
            initiative_value: e.initiative_value,
            is_current_turn: e.is_current_turn,
            portrait_url: None,
        })
        .collect();

    // Load recent chat (last 100 messages)
    let chat_rows: Vec<(ChatMessage, String)> = chat_messages::table
        .inner_join(users::table.on(users::id.eq(chat_messages::user_id)))
        .filter(chat_messages::session_id.eq(session_id))
        .order(chat_messages::id.desc())
        .limit(100)
        .select((ChatMessage::as_select(), users::username))
        .load(conn)
        .unwrap_or_default();

    let recent_chat: Vec<crate::models::ChatMessageInfo> = chat_rows
        .into_iter()
        .rev()
        .map(|(msg, uname)| crate::models::ChatMessageInfo {
            id: msg.id,
            username: uname,
            message: msg.message,
            is_dice_roll: msg.is_dice_roll,
            dice_result: msg.dice_result,
            created_at: msg.created_at,
        })
        .collect();

    // Load inventory
    let inventory = load_inventory(session_id);

    let initiative_locked = SESSION_MANAGER
        .sessions
        .get(&session_id)
        .map(|s| s.initiative_locked)
        .unwrap_or(false);

    crate::ws::messages::GameStateSnapshot {
        session_id,
        session_name,
        players,
        map: map_info,
        tokens: token_list,
        fog: fog_cells,
        initiative: initiative_list,
        recent_chat,
        inventory,
        initiative_locked,
    }
}

fn save_chat_message(
    session_id: i32,
    user_id: i32,
    username: &str,
    message: &str,
    is_dice_roll: bool,
    dice_result: Option<&str>,
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
        is_dice_roll,
        dice_result,
    };

    let _ = diesel::insert_into(chat_messages::table)
        .values(&new_msg)
        .execute(conn);

    let id: i32 = diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>(
        "last_insert_rowid()",
    ))
    .get_result(conn)
    .unwrap_or(0);

    crate::models::ChatMessageInfo {
        id,
        username: username.to_string(),
        message: message.to_string(),
        is_dice_roll,
        dice_result: dice_result.map(|s| s.to_string()),
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

fn place_token(
    session_id: i32,
    label: &str,
    x: f32,
    y: f32,
    color: &str,
    size: i32,
    creature_id: Option<i32>,
    image_url: Option<&str>,
) -> Result<crate::models::TokenInfo, String> {
    use crate::db;
    use crate::models::db_models::*;
    use crate::schema::*;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    // Get active map for this session
    let map_id: i32 = maps::table
        .filter(maps::session_id.eq(session_id))
        .order(maps::id.desc())
        .select(maps::id)
        .first(conn)
        .map_err(|_| "No active map for this session".to_string())?;

    let new_token = NewToken {
        map_id,
        label,
        x,
        y,
        color,
        size,
        visible: true,
        creature_id,
        image_url,
    };

    diesel::insert_into(tokens::table)
        .values(&new_token)
        .execute(conn)
        .map_err(|e| format!("Failed to place token: {e}"))?;

    let token_id: i32 = diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>(
        "last_insert_rowid()",
    ))
    .get_result(conn)
    .map_err(|e| format!("Failed to get token id: {e}"))?;

    // If linked to a creature, create a token instance with HP from the stat block
    let (current_hp, max_hp) = if let Some(cid) = creature_id {
        let creature: Creature = creatures::table
            .find(cid)
            .select(Creature::as_select())
            .first(conn)
            .map_err(|_| "Creature not found".to_string())?;

        let stat_data: serde_json::Value =
            serde_json::from_str(&creature.stat_data_json).unwrap_or_default();
        let hp = stat_data
            .get("hp_max")
            .and_then(|v| v.as_i64())
            .unwrap_or(10) as i32;

        let new_instance = NewTokenInstance {
            token_id,
            creature_id: cid,
            current_hp: hp,
            max_hp: hp,
            conditions_json: "[]".to_string(),
        };

        diesel::insert_into(token_instances::table)
            .values(&new_instance)
            .execute(conn)
            .map_err(|e| format!("Failed to create token instance: {e}"))?;

        (Some(hp), Some(hp))
    } else {
        (None, None)
    };

    Ok(crate::models::TokenInfo {
        id: token_id,
        label: label.to_string(),
        x,
        y,
        color: color.to_string(),
        size,
        visible: true,
        current_hp,
        max_hp,
        image_url: image_url.map(|s| s.to_string()),
        rotation: 0.0,
        conditions: vec![],
    })
}

fn remove_token(token_id: i32) {
    use crate::db;
    use crate::schema::{token_instances, tokens};
    use diesel::prelude::*;

    let conn = &mut db::get_conn();
    let _ = diesel::delete(token_instances::table.filter(token_instances::token_id.eq(token_id)))
        .execute(conn);
    let _ = diesel::delete(tokens::table.find(token_id)).execute(conn);
}

fn update_token_hp(token_id: i32, hp_change: i32) -> Result<(i32, i32), String> {
    use crate::db;
    use crate::models::db_models::TokenInstance;
    use crate::schema::token_instances;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    let instance: TokenInstance = token_instances::table
        .filter(token_instances::token_id.eq(token_id))
        .select(TokenInstance::as_select())
        .first(conn)
        .map_err(|_| "Token has no HP instance".to_string())?;

    let new_hp = (instance.current_hp + hp_change)
        .max(0)
        .min(instance.max_hp);

    diesel::update(token_instances::table.find(instance.id))
        .set(token_instances::current_hp.eq(new_hp))
        .execute(conn)
        .map_err(|e| format!("Failed to update HP: {e}"))?;

    Ok((new_hp, instance.max_hp))
}

fn load_map_with_tokens(
    map_id: i32,
) -> Result<
    (
        crate::models::MapInfo,
        Vec<crate::models::TokenInfo>,
        Vec<(i32, i32)>,
    ),
    String,
> {
    use crate::db;
    use crate::models::db_models::*;
    use crate::schema::*;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    let m: Map = maps::table
        .find(map_id)
        .select(Map::as_select())
        .first(conn)
        .map_err(|_| "Map not found".to_string())?;

    let map_info = crate::models::MapInfo {
        id: m.id,
        name: m.name,
        width: m.width,
        height: m.height,
        cell_size: m.cell_size,
        background_url: m.background_url,
    };

    let db_tokens: Vec<Token> = tokens::table
        .filter(tokens::map_id.eq(map_id))
        .select(Token::as_select())
        .load(conn)
        .unwrap_or_default();

    let token_list: Vec<crate::models::TokenInfo> = db_tokens
        .into_iter()
        .map(|t| {
            let instance: Option<TokenInstance> = token_instances::table
                .filter(token_instances::token_id.eq(t.id))
                .select(TokenInstance::as_select())
                .first(conn)
                .optional()
                .unwrap_or(None);

            let conditions: Vec<String> = instance
                .as_ref()
                .and_then(|i| serde_json::from_str(&i.conditions_json).ok())
                .unwrap_or_default();

            crate::models::TokenInfo {
                id: t.id,
                label: t.label,
                x: t.x,
                y: t.y,
                color: t.color,
                size: t.size,
                visible: t.visible,
                current_hp: instance.as_ref().map(|i| i.current_hp),
                max_hp: instance.as_ref().map(|i| i.max_hp),
                image_url: t.image_url,
                rotation: t.rotation,
                conditions,
            }
        })
        .collect();

    let fog_cells: Vec<(i32, i32)> = fog_of_war::table
        .filter(fog_of_war::map_id.eq(map_id))
        .select((fog_of_war::x, fog_of_war::y))
        .load(conn)
        .unwrap_or_default();

    Ok((map_info, token_list, fog_cells))
}

fn save_initiative(session_id: i32, entries: &[crate::models::InitiativeEntryInfo]) {
    use crate::db;
    use crate::models::db_models::NewInitiativeEntry;
    use crate::schema::initiative;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    // Replace all initiative entries for this session
    let _ = diesel::delete(initiative::table.filter(initiative::session_id.eq(session_id)))
        .execute(conn);

    for (i, entry) in entries.iter().enumerate() {
        let new_entry = NewInitiativeEntry {
            session_id,
            label: &entry.label,
            initiative_value: entry.initiative_value,
            is_current_turn: entry.is_current_turn,
            sort_order: i as i32,
        };
        let _ = diesel::insert_into(initiative::table)
            .values(&new_entry)
            .execute(conn);
    }
}

fn update_character_field(
    character_id: i32,
    user_id: i32,
    field_path: &str,
    value: &serde_json::Value,
) -> Result<(), String> {
    use crate::db;
    use crate::models::db_models::Character;
    use crate::schema::characters;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    let character: Character = characters::table
        .find(character_id)
        .select(Character::as_select())
        .first(conn)
        .map_err(|_| "Character not found".to_string())?;

    // Players can only edit their own characters
    if character.user_id != user_id {
        return Err("You can only edit your own character".to_string());
    }

    let mut data: serde_json::Value =
        serde_json::from_str(&character.data_json).unwrap_or(serde_json::json!({}));

    // Set the field at the given path (supports dot-separated paths like "stats.strength")
    let parts: Vec<&str> = field_path.split('.').collect();
    let mut current = &mut data;
    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            current[part] = value.clone();
        } else {
            if !current.get(part).is_some_and(|v| v.is_object()) {
                current[part] = serde_json::json!({});
            }
            current = &mut current[part];
        }
    }

    let json_str = serde_json::to_string(&data).map_err(|e| format!("Serialization error: {e}"))?;

    diesel::update(characters::table.find(character_id))
        .set(characters::data_json.eq(json_str))
        .execute(conn)
        .map_err(|e| format!("Failed to update character: {e}"))?;

    Ok(())
}

fn add_inventory_item(
    session_id: i32,
    name: &str,
    description: &str,
    quantity: i32,
    is_party_item: bool,
) -> Result<Vec<crate::models::InventoryItemInfo>, String> {
    use crate::db;
    use crate::models::db_models::NewInventoryItem;
    use crate::schema::inventory_items;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    let new_item = NewInventoryItem {
        session_id,
        name,
        description,
        quantity,
        is_party_item,
    };

    diesel::insert_into(inventory_items::table)
        .values(&new_item)
        .execute(conn)
        .map_err(|e| format!("Failed to add inventory item: {e}"))?;

    Ok(load_inventory(session_id))
}

fn remove_inventory_item(item_id: i32) {
    use crate::db;
    use crate::schema::inventory_items;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();
    let _ = diesel::delete(inventory_items::table.find(item_id)).execute(conn);
}

fn update_inventory_item(
    item_id: i32,
    name: Option<&str>,
    description: Option<&str>,
    quantity: Option<i32>,
) {
    use crate::db;
    use crate::schema::inventory_items;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    if let Some(name) = name {
        let _ = diesel::update(inventory_items::table.find(item_id))
            .set(inventory_items::name.eq(name))
            .execute(conn);
    }
    if let Some(description) = description {
        let _ = diesel::update(inventory_items::table.find(item_id))
            .set(inventory_items::description.eq(description))
            .execute(conn);
    }
    if let Some(quantity) = quantity {
        let _ = diesel::update(inventory_items::table.find(item_id))
            .set(inventory_items::quantity.eq(quantity))
            .execute(conn);
    }
}

fn load_inventory(session_id: i32) -> Vec<crate::models::InventoryItemInfo> {
    use crate::db;
    use crate::models::db_models::InventoryItem;
    use crate::schema::inventory_items;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    let items: Vec<InventoryItem> = inventory_items::table
        .filter(inventory_items::session_id.eq(session_id))
        .select(InventoryItem::as_select())
        .load(conn)
        .unwrap_or_default();

    items
        .into_iter()
        .map(|item| crate::models::InventoryItemInfo {
            id: item.id,
            name: item.name,
            description: item.description,
            quantity: item.quantity,
            is_party_item: item.is_party_item,
        })
        .collect()
}

/// D&D 5e ability modifier: floor((score - 10) / 2)
fn ability_modifier(score: f64) -> i32 {
    ((score - 10.0) / 2.0).floor() as i32
}

/// Get the total initiative modifier for a character (dex mod + initiative_bonus).
fn get_initiative_modifier(session_id: i32, character_id: i32) -> i32 {
    use crate::db;
    use crate::schema::characters;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();
    let data_json: String = characters::table
        .find(character_id)
        .filter(characters::session_id.eq(session_id))
        .select(characters::data_json)
        .first(conn)
        .unwrap_or_default();

    let data: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&data_json).unwrap_or_default();

    let dex = data
        .get("dexterity")
        .and_then(|v| v.as_f64())
        .unwrap_or(10.0);
    let init_bonus = data
        .get("initiative_bonus")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as i32;
    ability_modifier(dex) + init_bonus
}

/// Get the total initiative modifier for a creature (dex mod + initiative_bonus from stat_data).
fn get_creature_initiative_modifier(_session_id: i32, creature_id: i32) -> i32 {
    use crate::db;
    use crate::schema::creatures;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();
    let stat_json: String = creatures::table
        .find(creature_id)
        .select(creatures::stat_data_json)
        .first(conn)
        .unwrap_or_default();

    let data: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&stat_json).unwrap_or_default();

    let dex = data
        .get("dexterity")
        .and_then(|v| v.as_f64())
        .unwrap_or(10.0);
    let init_bonus = data
        .get("initiative_bonus")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as i32;
    ability_modifier(dex) + init_bonus
}

/// Save initiative, update cache, broadcast result + chat message.
fn broadcast_initiative_roll(
    session_id: i32,
    user_id: i32,
    username: &str,
    result: InitiativeRollResult,
) {
    save_initiative(session_id, &result.entries);
    if let Some(mut session) = SESSION_MANAGER.sessions.get_mut(&session_id) {
        session.initiative_order = result.entries.clone();
    }
    SESSION_MANAGER.broadcast(
        session_id,
        &ServerMessage::InitiativeUpdated {
            entries: result.entries,
        },
        None,
    );
    let mod_str = if result.modifier >= 0 {
        format!("+{}", result.modifier)
    } else {
        format!("{}", result.modifier)
    };
    let _ = save_chat_message(
        session_id,
        user_id,
        username,
        &format!(
            "{} rolled initiative: [{}]{} = {}",
            result.label, result.d20, mod_str, result.total
        ),
        true,
        Some(&format!(
            "{{\"rolls\":[{}],\"total\":{}}}",
            result.d20, result.total
        )),
    );
    SESSION_MANAGER.broadcast(
        session_id,
        &ServerMessage::DiceResult {
            username: username.to_string(),
            expression: format!("d20{mod_str} (initiative)"),
            rolls: vec![result.d20],
            total: result.total,
        },
        None,
    );
}

/// Initiative roll result with breakdown for chat logging.
struct InitiativeRollResult {
    label: String,
    d20: i32,
    modifier: i32,
    total: i32,
    entries: Vec<crate::models::InitiativeEntryInfo>,
}

/// Roll initiative for a character: d20 + dex mod + initiative bonus.
fn roll_character_initiative(
    session_id: i32,
    character_id: i32,
) -> Result<InitiativeRollResult, String> {
    use crate::db;
    use crate::schema::characters;
    use diesel::prelude::*;
    use rand::Rng;

    let conn = &mut db::get_conn();

    let (char_name, portrait_url): (String, Option<String>) = characters::table
        .find(character_id)
        .filter(characters::session_id.eq(session_id))
        .select((characters::name, characters::portrait_url))
        .first(conn)
        .map_err(|_| "Character not found in this session".to_string())?;

    let modifier = get_initiative_modifier(session_id, character_id);
    let mut rng = rand::thread_rng();
    let d20: i32 = rng.gen_range(1..=20);
    let total = d20 + modifier;

    let mut entries = SESSION_MANAGER
        .sessions
        .get(&session_id)
        .map(|s| s.initiative_order.clone())
        .unwrap_or_default();

    // Remove existing entry for this character (by label match)
    entries.retain(|e| e.label != char_name);

    let is_first = entries.is_empty();
    entries.push(crate::models::InitiativeEntryInfo {
        id: 0,
        label: char_name.clone(),
        initiative_value: total as f32,
        is_current_turn: is_first,
        portrait_url,
    });

    entries.sort_by(|a, b| b.initiative_value.partial_cmp(&a.initiative_value).unwrap());

    Ok(InitiativeRollResult {
        label: char_name,
        d20,
        modifier,
        total,
        entries,
    })
}

/// Roll initiative for a creature: d20 + dex mod + initiative bonus.
fn roll_creature_initiative(
    session_id: i32,
    creature_id: i32,
    label: &str,
) -> Result<InitiativeRollResult, String> {
    use crate::db;
    use crate::schema::creatures;
    use diesel::prelude::*;
    use rand::Rng;

    let conn = &mut db::get_conn();
    let image_url: Option<String> = creatures::table
        .find(creature_id)
        .select(creatures::image_url)
        .first::<Option<String>>(conn)
        .ok()
        .flatten();

    let modifier = get_creature_initiative_modifier(session_id, creature_id);
    let mut rng = rand::thread_rng();
    let d20: i32 = rng.gen_range(1..=20);
    let total = d20 + modifier;

    let mut entries = SESSION_MANAGER
        .sessions
        .get(&session_id)
        .map(|s| s.initiative_order.clone())
        .unwrap_or_default();

    let is_first = entries.is_empty();
    // Don't remove existing — creatures can have multiple entries (e.g. 5 goblins)
    entries.push(crate::models::InitiativeEntryInfo {
        id: 0,
        label: label.to_string(),
        initiative_value: total as f32,
        is_current_turn: is_first,
        portrait_url: image_url,
    });

    entries.sort_by(|a, b| b.initiative_value.partial_cmp(&a.initiative_value).unwrap());

    Ok(InitiativeRollResult {
        label: label.to_string(),
        d20,
        modifier,
        total,
        entries,
    })
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
