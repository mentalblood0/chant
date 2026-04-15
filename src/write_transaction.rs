use std::collections::BTreeMap;
use std::collections::BTreeSet;

use anyhow::{anyhow, Result};
use fallible_iterator::FallibleIterator;
use trove::path_segments;

use crate::commands::Command;
use crate::define_read_methods;
use crate::read_transaction::ReadTransactionMethods;
use crate::sweater;
use crate::user::{MessageGlobalId, QueuedCommands, Role, User};
use wool::{
    content::Content,
    graph_generator::{GraphGenerator, GraphGeneratorConfig},
    text::Entity,
    thesis::Thesis,
};

pub struct WriteTransaction<'a, 'b, 'c, 'd, 'e> {
    pub sweater_transaction: &'a mut sweater::WriteTransaction<'b, 'c, 'd, 'e>,
}

impl<'a, 'b, 'c, 'd, 'e> ReadTransactionMethods<'a> for WriteTransaction<'a, 'b, 'c, 'd, 'e> {
    define_read_methods!('a);
}

impl<'a, 'b, 'c, 'd, 'e> ReadTransactionMethods<'a> for &mut WriteTransaction<'a, 'b, 'c, 'd, 'e> {
    define_read_methods!('a);
}

impl WriteTransaction<'_, '_, '_, '_, '_> {
    pub fn queue_commands(
        &mut self,
        source_message_id: MessageGlobalId,
        user_id: trove::DocumentId,
        text: &str,
    ) -> Result<()> {
        let commands = serde_json::to_value(Some(QueuedCommands {
            source_message_global_id: source_message_id,
            commands: {
                let mut aliases_resolver = sweater::LocalAliasesResolver {
                    read_able_transaction: self.sweater_transaction,
                    known_aliases: BTreeMap::new(),
                };
                let mut result = vec![];
                for line in text.lines() {
                    result.push(wool::command::Command::parse(
                        line,
                        &mut aliases_resolver,
                        &self
                            .sweater_transaction
                            .sweater_config
                            .supported_relations_kinds,
                    )?);
                }
                result
            },

            sent_to_cantors_messages_ids: vec![],
        }))?;
        self.sweater_transaction.chest_transaction.users_set(
            user_id,
            trove::path_segments!("commands_queue"),
            commands,
        )?;
        Ok(())
    }

    pub fn execute_commands_queue(&mut self, user_telegram_id: i64) -> Result<()> {
        let user_id = user_telegram_id.into();
        if let Some(commands_json_value) = self
            .sweater_transaction
            .chest_transaction
            .users_get(&user_id, &trove::path_segments!("commands_queue"))?
        {
            let queued_commands = serde_json::from_value::<QueuedCommands>(commands_json_value)?;
            if !queued_commands.commands.is_empty() {
                for command in queued_commands.commands {
                    self.sweater_transaction.execute_command(&command)?;
                }
                self.sweater_transaction
                    .chest_transaction
                    .users_remove(&user_id, &trove::path_segments!("commands_queue"))?;
            }
        }
        Ok(())
    }

    pub fn add_users(&mut self, users: &Vec<User>) -> Result<()> {
        for user in users {
            self.sweater_transaction
                .chest_transaction
                .users_insert_with_id(trove::Document {
                    id: user.telegram_id.into(),
                    value: serde_json::to_value(user)?,
                })?;
        }
        Ok(())
    }

    pub fn execute_command(&mut self, command: &Command) -> Result<String> {
        match command {
            Command::GetThesisByReference(thesis_id) => Ok(
                if let Some(thesis) =
                    wool::read_transaction_methods::ReadTransactionMethods::get_thesis(
                        self.sweater_transaction,
                        thesis_id,
                    )?
                {
                    self.format_thesis(&thesis)?
                } else {
                    "Not found".to_string()
                },
            ),
            Command::GetAllTags => {
                let mut result = BTreeSet::new();
                for tags in wool::read_transaction_methods::ReadTransactionMethods::iter_theses(
                    self.sweater_transaction,
                )?
                .map(|thesis| Ok(thesis.tags))
                .collect::<Vec<_>>()?
                {
                    for tag in tags {
                        result.insert(tag);
                    }
                }
                Ok(result
                    .iter()
                    .map(|tag| telegram_escape::tg_escape(&self.format_tag(&tag.0)))
                    .collect::<Vec<_>>()
                    .join(" "))
            }
            Command::GetSupportedRelationsKinds => Ok(self
                .sweater_transaction
                .sweater_config
                .supported_relations_kinds
                .iter()
                .cloned()
                .map(|relation_kind| relation_kind.0)
                .collect::<Vec<_>>()
                .join(", ")),
            Command::GetThesesByTags(tags) => Ok(
                wool::read_transaction_methods::ReadTransactionMethods::iter_theses_ids_by_tags(
                    self.sweater_transaction,
                    &tags,
                    &vec![],
                    None,
                )?
                .map(|thesis_id| {
                    self.format_thesis(
                        &wool::read_transaction_methods::ReadTransactionMethods::get_thesis(
                            self.sweater_transaction,
                            &thesis_id,
                        )?
                        .ok_or(anyhow!(
                            "Thesis with identifier {:?} not found",
                            thesis_id.to_string()
                        ))?,
                    )
                })
                .collect::<Vec<_>>()?
                .join("\n\n"),
            ),
            Command::GetThesesByWords(words) => Ok(
                wool::read_transaction_methods::ReadTransactionMethods::iter_theses_ids_by_entities(
                    self.sweater_transaction,
                    &words.iter().map(|word| Entity::Word(word.clone())).collect(),
                    &vec![],
                    None,
                )?
                .map(|thesis_id| {
                    self.format_thesis(
                        &wool::read_transaction_methods::ReadTransactionMethods::get_thesis(
                            self.sweater_transaction,
                            &thesis_id,
                        )?
                        .ok_or(anyhow!(
                            "Thesis with identifier {:?} not found",
                            thesis_id.to_string()
                        ))?,
                    )
                })
                .collect::<Vec<_>>()?
                .join("\n\n"),
            ),
            Command::AddOfferers(users) => {
                self.add_users(users)?;
                Ok("Added".to_string())
            }
            Command::PromoteToCantor(user_id) => {
                self.sweater_transaction.chest_transaction.users_set(
                    user_id.clone(),
                    trove::path_segments!("role"),
                    serde_json::to_value(Role::Cantor)?,
                )?;
                Ok("Promoted".to_string())
            }
        }
    }
}
