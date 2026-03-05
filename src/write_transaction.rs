use std::collections::BTreeMap;

use anyhow::{Context, Result};
use fallible_iterator::FallibleIterator;

use crate::define_read_methods;
use crate::read_transaction::ReadTransactionMethods;
use crate::sweater;
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
    pub fn queue_commands(&mut self, user_id: trove::DocumentId, text: &str) -> Result<()> {
        let commands = sweater::CommandsIterator::new(
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
        .collect::<Vec<_>>()?;
        self.sweater_transaction.chest_transaction.users_update(
            user_id,
            trove::path_segments!("commands_queue"),
            serde_json::to_value(commands)?,
        )?;
        Ok(())
    }

    pub fn execute_commands_queue(&mut self, user_telegram_id: i64) -> Result<()> {
        let user_id = self
            .get_user_id_by_telegram_id(user_telegram_id)?
            .with_context(|| {
                "Can not execute commands queue for user with telegram id {telegram_id:?} as there \
                 is no such user"
            })?;
        if let Some(commands_json_value) = self
            .sweater_transaction
            .chest_transaction
            .users_get(&user_id, &trove::path_segments!("commands_queue"))?
        {
            let commands = serde_json::from_value::<Vec<sweater::Command>>(commands_json_value)?;
            for command in commands {
                self.sweater_transaction.execute_command(&command)?;
            }
            self.sweater_transaction
                .chest_transaction
                .users_remove(&user_id, &trove::path_segments!("commands_queue"))?;
        }
        Ok(())
    }

    pub fn add_users(&mut self, users: &Vec<User>) -> Result<()> {
        for user in users {
            self.sweater_transaction
                .chest_transaction
                .users_insert_with_id(trove::Document {
                    id: user.id(),
                    value: serde_json::to_value(user)?,
                })?;
        }
        Ok(())
    }
}
