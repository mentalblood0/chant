use crate::sweater;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum Role {
    Cantor,
    Offerer,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct MessageGlobalId {
    pub message_id: i32,
    pub chat_id: i64,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct QueuedCommands {
    pub source_message_global_id: MessageGlobalId,
    pub commands: Vec<sweater::Command>,
    pub sent_to_cantors_messages_ids: Vec<MessageGlobalId>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub telegram_id: i64,
    pub role: Role,

    #[serde(default)]
    pub commands_queue: Option<QueuedCommands>,
}
