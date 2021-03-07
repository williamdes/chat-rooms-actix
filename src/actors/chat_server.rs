use actix::{Actor, Context, Handler, MessageResult, Recipient};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::messages::{
    chat_server::{ClientMessage, Connect, Disconnect, JoinRoom},
    chat_session::Message,
};
use crate::models::SessionId;

pub struct ChatServer {
    sessions: HashMap<SessionId, Recipient<Message>>,
    rooms: HashMap<String, HashSet<SessionId>>,
}

impl ChatServer {
    pub fn new() -> Self {
        ChatServer {
            sessions: HashMap::new(),
            rooms: HashMap::new(),
        }
    }

    pub fn send_message(&self, room: &str, message: &str, skip_id: &Uuid) {
        self.rooms.get(room).map(|sessions| {
            sessions.iter().for_each(|id| {
                if id != skip_id {
                    self.sessions
                        .get(id)
                        .map(|addr| addr.do_send(Message(message.into())));
                }
            });
        });
    }
}

impl Actor for ChatServer {
    type Context = Context<Self>;
}

impl Handler<Connect> for ChatServer {
    type Result = MessageResult<Connect>;

    fn handle(&mut self, msg: Connect, _ctx: &mut Self::Context) -> Self::Result {
        let session_id = Uuid::new_v4();
        self.sessions.insert(session_id, msg.addr);
        MessageResult(session_id)
    }
}

impl Handler<Disconnect> for ChatServer {
    type Result = ();

    fn handle(
        &mut self,
        Disconnect { session }: Disconnect,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        for (_id, sessions) in self.rooms.iter_mut() {
            sessions.remove(&session);
        }
        let _ = self.sessions.remove(&session);
    }
}

impl Handler<ClientMessage> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: ClientMessage, _ctx: &mut Self::Context) -> Self::Result {
        let ClientMessage { session, room, msg } = msg;
        self.send_message(&room, &msg, &session);
    }
}

impl Handler<JoinRoom> for ChatServer {
    type Result = ();
    fn handle(
        &mut self,
        JoinRoom { session, room }: JoinRoom,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let mut rooms = Vec::new();

        // remove session from all rooms
        for (id, sessions) in &mut self.rooms {
            if sessions.remove(&session) {
                rooms.push(id.to_owned());
            }
        }
        // send message to other users
        for room in rooms {
            self.send_message(&room, "Someone disconnected", &session);
        }

        self.rooms
            .entry(room.clone())
            .or_insert_with(HashSet::new)
            .insert(session);

        self.send_message(&room, "Someone connected", &session);
    }
}
