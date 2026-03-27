use std::collections::BTreeMap;

use anyhow::Result;
use fallible_iterator::FallibleIterator;
use trove::path_segments;

use crate::commands::Command;
use crate::define_read_methods;
use crate::read_transaction::ReadTransactionMethods;
use crate::sweater;
use crate::user::MessageGlobalId;
use crate::user::QueuedCommands;
use crate::user::Role;
use crate::user::User;

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
        self.sweater_transaction.chest_transaction.users_set(
            user_id,
            trove::path_segments!("commands_queue"),
            serde_json::to_value(Some(QueuedCommands {
                source_message_global_id: source_message_id,
                commands: sweater::CommandsIterator::new(
                    text,
                    &self
                        .sweater_transaction
                        .sweater_config
                        .supported_relations_kinds,
                    &mut sweater::AliasesResolver {
                        read_able_transaction: self.sweater_transaction,
                        known_aliases: BTreeMap::new(),
                    },
                )
                .collect::<Vec<_>>()?,
                sent_to_cantors_messages_ids: vec![],
            }))?,
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

    pub fn execute_command(&self, command: &Command) -> Result<impl serde::Serialize> {
        match command {
            Command::GetTheses(theses_ids) => Ok(fallible_iterator::convert(
                theses_ids.iter().map(|thesis_id| {
                    sweater::ReadTransactionMethods::get_thesis(self.sweater_transaction, thesis_id)
                }),
            )
            .collect::<Vec<_>>()?),
        }
    }
}
