use crate::sweater;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Role {
    Cantor,
    Offerer,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub telegram_id: i64,
    pub role: Role,

    #[serde(default)]
    pub commands_queue: Vec<sweater::Command>,
}

impl User {
    pub fn id_from_telegram_id(telegram_id: i64) -> trove::DocumentId {
        let source: Vec<u8> = telegram_id.to_string().bytes().collect();
        trove::DocumentId {
            value: xxhash_rust::xxh3::xxh3_128(&source).to_be_bytes(),
        }
    }

    pub fn id(&self) -> trove::DocumentId {
        Self::id_from_telegram_id(self.telegram_id)
    }
}
