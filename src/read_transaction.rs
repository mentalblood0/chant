use anyhow::Result;
use fallible_iterator::FallibleIterator;
use trove::path_segments;

use crate::sweater;
use crate::user::Role;
use wool::{
    content::Content,
    graph_generator::{GraphGenerator, GraphGeneratorConfig},
    thesis::Thesis,
};

pub struct ReadTransaction<'a> {
    pub sweater_transaction: &'a sweater::ReadTransaction<'a>,
    pub target_bot_name_for_deeplinks_formatting: String,
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

        fn get_graph_definition(&self) -> Result<String> {
            Ok(GraphGenerator::new(
                &GraphGeneratorConfig { wrap_width: 64 },
                self.sweater_transaction,
            )?
            .collect::<Vec<_>>()?
            .join(""))
        }

        fn format_thesis_id(&self, thesis_id: &trove::DocumentId) -> Result<String> {
            let thesis_id_string = thesis_id.to_string();
            Ok(
                if let Some(alias) =
                    &wool::read_transaction_methods::ReadTransactionMethods::get_alias_by_thesis_id(
                        self.sweater_transaction,
                        &thesis_id,
                    )?
                {
                    format!(
                        "[{}](https://t.me/{}?start=reference_{})",
                        alias.0, self.target_bot_name_for_deeplinks_formatting, thesis_id_string
                    )
                } else {
                    format!(
                        "[{}](https://t.me/{}?start=reference_{})",
                        thesis_id_string,
                        self.target_bot_name_for_deeplinks_formatting,
                        thesis_id_string
                    )
                },
            )
        }

        fn format_tag(&self, tag_text: &str) -> String {
            format!(
                "[{}](https://t.me/{}?start=tags_{})",
                tag_text, self.target_bot_name_for_deeplinks_formatting, tag_text
            )
        }

        fn format_thesis(&self, thesis: &Thesis) -> Result<String> {
            let mut result = format!(
                "id: `{}`",
                telegram_escape::tg_escape(&thesis.id().to_string())
            );
            if let Some(ref alias) = thesis.alias {
                result.push_str(&format!(
                    "\nalias: `{}`",
                    telegram_escape::tg_escape(&alias.0)
                ));
            }
            result.push_str(&format!(
                "\ncontent: {}",
                telegram_escape::tg_escape(&match thesis.content {
                    Content::Text(ref text) => text.composed(
                        |referenced_thesis_id| self.format_thesis_id(referenced_thesis_id)
                    )?,
                    Content::Relation(ref relation) => format!(
                        "{} {} {}",
                        self.format_thesis_id(&relation.from)?,
                        relation.kind.0,
                        self.format_thesis_id(&relation.to)?,
                    )
                    .to_string(),
                })
            ));
            let tags_text = telegram_escape::tg_escape(
                &thesis
                    .tags
                    .iter()
                    .cloned()
                    .map(|tag| self.format_tag(&tag.0))
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            if !tags_text.is_empty() {
                result.push_str(&format!("\ntagged with {}", tags_text));
            }
            let references_text = telegram_escape::tg_escape(
                &fallible_iterator::convert(
                    wool::read_transaction_methods::ReadTransactionMethods::where_referenced(
                        self.sweater_transaction,
                        &thesis.id(),
                    )?
                    .into_iter()
                    .map(|thesis_id| self.format_thesis_id(&thesis_id)),
                )
                .collect::<Vec<_>>()?
                .join(", "),
            );
            if !references_text.is_empty() {
                result.push_str(&format!("\nreferenced in {}", references_text));
            }
            Ok(result)
        }
    };
}

pub trait ReadTransactionMethods<'a> {
    fn is_queue_full(&self, user_telegram_id: i64) -> Result<bool>;
    fn get_cantors_user_ids(&self) -> Result<Vec<trove::DocumentId>>;
    fn get_graph_definition(&self) -> Result<String>;
    fn format_thesis_id(&self, thesis_id: &trove::DocumentId) -> Result<String>;
    fn format_tag(&self, tag_text: &str) -> String;
    fn format_thesis(&self, thesis: &Thesis) -> Result<String>;
}

impl<'a> ReadTransactionMethods<'a> for ReadTransaction<'a> {
    define_read_methods!('a);
}
