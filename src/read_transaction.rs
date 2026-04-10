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

        fn format_reference(&self, reference_text: &String) -> String {
            format!(
                "[{}](https://t.me/mentalblood_test_bot?start=reference_{})",
                reference_text, reference_text
            )
        }

        fn format_tag(&self, tag_text: &String) -> String {
            format!(
                "[{}](https://t.me/mentalblood_test_bot?start=tags_{})",
                tag_text, tag_text
            )
        }

        fn format_thesis(&self, thesis: &sweater::Thesis) -> Result<String> {
            Ok(format!(
                "{}\ntags: {}\nreferenced in: {}",
                telegram_escape::tg_escape(&match thesis.content {
                    sweater::Content::Text(ref text) => text.composed(),
                    sweater::Content::Relation(ref relation) => format!(
                        "{} {} {}",
                        self.format_reference(&relation.from.to_string()),
                        relation.kind.0,
                        self.format_reference(&relation.to.to_string()),
                    )
                    .to_string(),
                }),
                telegram_escape::tg_escape(
                    &thesis
                        .tags
                        .iter()
                        .cloned()
                        .map(|tag| self.format_tag(&tag.0))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                telegram_escape::tg_escape(
                    &sweater::ReadTransactionMethods::where_referenced(
                        self.sweater_transaction,
                        &thesis.id()?
                    )?
                    .into_iter()
                    .map(|thesis_id| self.format_reference(&thesis_id.to_string()))
                    .collect::<Vec<_>>()
                    .join(", ")
                )
            ))
        }
    };
}

pub trait ReadTransactionMethods<'a> {
    fn is_queue_full(&self, user_telegram_id: i64) -> Result<bool>;
    fn get_cantors_user_ids(&self) -> Result<Vec<trove::DocumentId>>;
    fn format_thesis(&self, thesis: &sweater::Thesis) -> Result<String>;
    fn format_reference(&self, reference_text: &String) -> String;
    fn format_tag(&self, tag_text: &String) -> String;
}

impl<'a> ReadTransactionMethods<'a> for ReadTransaction<'a> {
    define_read_methods!('a);
}
