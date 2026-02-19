#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Role {
    Cantor,
    Offerer,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub telegram_id: String,
    pub role: Role,

    #[serde(default)]
    pub commands_queue: Vec<woollib::commands::Command>,
}

impl User {
    pub fn id(&self) -> trove::ObjectId {
        let source: Vec<u8> = self.telegram_id.bytes().collect();
        trove::ObjectId {
            value: xxhash_rust::xxh3::xxh3_128(&source).to_be_bytes(),
        }
    }
}
