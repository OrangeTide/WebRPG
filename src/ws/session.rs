use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use tokio::sync::mpsc;

use crate::ws::messages::ServerMessage;

pub static SESSION_MANAGER: Lazy<SessionManager> = Lazy::new(SessionManager::new);

pub struct SessionManager {
    pub sessions: DashMap<i32, ActiveSession>,
}

pub struct ActiveSession {
    pub clients: DashMap<String, mpsc::UnboundedSender<ServerMessage>>,
    pub active_map_id: Option<i32>,
    pub token_positions: HashMap<i32, (f32, f32)>,
    pub revealed_fog: HashSet<(i32, i32)>,
    pub initiative_order: Vec<crate::models::InitiativeEntryInfo>,
    pub initiative_locked: bool,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
        }
    }

    pub fn get_or_create_session(
        &self,
        session_id: i32,
    ) -> dashmap::mapref::one::RefMut<'_, i32, ActiveSession> {
        self.sessions
            .entry(session_id)
            .or_insert_with(|| ActiveSession {
                clients: DashMap::new(),
                active_map_id: None,
                token_positions: HashMap::new(),
                revealed_fog: HashSet::new(),
                initiative_order: Vec::new(),
                initiative_locked: false,
            })
    }

    pub fn broadcast(&self, session_id: i32, message: &ServerMessage, exclude: Option<&str>) {
        if let Some(session) = self.sessions.get(&session_id) {
            for entry in session.clients.iter() {
                if exclude.is_some_and(|ex| ex == entry.key()) {
                    continue;
                }
                let _ = entry.value().send(message.clone());
            }
        }
    }

    /// Send a message to a single client by username, if connected. Returns
    /// true if a connected client received it.
    pub fn send_to(&self, session_id: i32, username: &str, message: &ServerMessage) -> bool {
        if let Some(session) = self.sessions.get(&session_id)
            && let Some(tx) = session.clients.get(username)
        {
            return tx.send(message.clone()).is_ok();
        }
        false
    }

    pub fn add_client(
        &self,
        session_id: i32,
        username: String,
        tx: mpsc::UnboundedSender<ServerMessage>,
    ) {
        let session = self.get_or_create_session(session_id);
        session.clients.insert(username, tx);
    }

    pub fn remove_client(&self, session_id: i32, username: &str) {
        if let Some(session) = self.sessions.get(&session_id) {
            session.clients.remove(username);
            if session.clients.is_empty() {
                drop(session);
                self.sessions.remove(&session_id);
            }
        }
    }
}
