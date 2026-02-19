use std::collections::BTreeMap;

use anyhow::Result;
use fallible_iterator::FallibleIterator;

use crate::define_read_methods;
use crate::read_transaction::ReadTransactionMethods;
use crate::user::User;

pub struct WriteTransaction<'a, 'b, 'c, 'd, 'e> {
    pub sweater_transaction: &'a mut woollib::write_transaction::WriteTransaction<'b, 'c, 'd, 'e>,
}

impl<'a, 'b, 'c, 'd, 'e> ReadTransactionMethods<'a> for WriteTransaction<'a, 'b, 'c, 'd, 'e> {
    define_read_methods!('a);
}

impl<'a, 'b, 'c, 'd, 'e> ReadTransactionMethods<'a> for &mut WriteTransaction<'a, 'b, 'c, 'd, 'e> {
    define_read_methods!('a);
}

impl WriteTransaction<'_, '_, '_, '_, '_> {
    pub fn queue_commands(&mut self, text: &str) -> Result<()> {
        let commands = woollib::commands::CommandsIterator::new(
            text,
            &self
                .sweater_transaction
                .sweater_config
                .supported_relations_kinds,
            &mut woollib::aliases_resolver::AliasesResolver {
                read_able_transaction: self.sweater_transaction,
                known_aliases: BTreeMap::new(),
            },
        )
        .collect::<Vec<_>>()?;
        for command in &commands {
            println!("executing {command:?}");
            self.sweater_transaction.execute_command(&command)?;
        }
        Ok(())
    }

    pub fn add_users(&mut self, users: &Vec<User>) -> Result<()> {
        for user in users {
            self.sweater_transaction
                .chest_transaction
                .insert_with_id(trove::Object {
                    id: user.id(),
                    value: serde_json::to_value(user)?,
                })?;
        }
        Ok(())
    }
}
