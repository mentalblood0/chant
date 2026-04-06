use anyhow::Result;
use fallible_iterator::FallibleIterator;
use trove::path_segments;

use crate::sweater;
use crate::user::Role;

pub struct ReadTransaction<'a> {
    pub sweater_transaction: &'a sweater::ReadTransaction<'a>,
}

#[macro_export]
macro_rules! define_read_methods {
    ($lifetime:lifetime) => {
        fn is_queue_full(&self, user_telegram_id: i64) -> Result<bool> {
            self.sweater_transaction
                .chest_transaction
                .theses_contains_path(&user_telegram_id.into(), &path_segments!("commands_queue"))
        }

        fn get_cantors_user_ids(&self) -> Result<Vec<trove::DocumentId>> {
            self.sweater_transaction
                .chest_transaction
                .users_select(
                    &vec![(
                        trove::search_path_segments!("role"),
                        serde_json::to_value(Role::Cantor)?,
                    )],
                    &vec![],
                    None,
                )?
                .collect()
        }

        fn format_thesis(&self, thesis: &sweater::Thesis) -> Result<String> {
            Ok(format!(
                "{}\ntags:\n{}\nreferenced in:\n{}",
                match thesis.content {
                    sweater::Content::Text(ref text) => text.composed(),
                    sweater::Content::Relation(ref relation) => format!(
                        "{} {} {}",
                        relation.from.to_string(),
                        relation.kind.0,
                        relation.to.to_string()
                    )
                    .to_string(),
                },
                thesis
                    .tags
                    .iter()
                    .cloned()
                    .map(|tag| tag.0)
                    .collect::<Vec<_>>()
                    .join("\n"),
                sweater::ReadTransactionMethods::where_referenced(
                    self.sweater_transaction,
                    &thesis.id()?
                )?
                .into_iter()
                .map(|thesis_id| thesis_id.to_string())
                .collect::<Vec<_>>()
                .join("\n")
            ))
        }
    };
}

pub trait ReadTransactionMethods<'a> {
    fn is_queue_full(&self, user_telegram_id: i64) -> Result<bool>;
    fn get_cantors_user_ids(&self) -> Result<Vec<trove::DocumentId>>;
    fn format_thesis(&self, thesis: &sweater::Thesis) -> Result<String>;
}

impl<'a> ReadTransactionMethods<'a> for ReadTransaction<'a> {
    define_read_methods!('a);
}
