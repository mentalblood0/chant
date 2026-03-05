use crate::sweater;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Role {
    Cantor,
    Offerer,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct MessageId {
    pub telegram_message_id: i32,
    pub chat_id: i64,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct QueuedCommands {
    #[serde(default)]
    pub commands: Vec<sweater::Command>,

    #[serde(default)]
    pub sent_to_cantors_messages_ids: Vec<MessageId>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub telegram_id: i64,
    pub role: Role,
    pub commands_queue: QueuedCommands,
}
