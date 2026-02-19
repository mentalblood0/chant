use anyhow::Result;
use fallible_iterator::FallibleIterator;

use crate::user::Role;

pub struct ReadTransaction<'a> {
    pub sweater_transaction: &'a woollib::read_transaction::ReadTransaction<'a>,
}

#[macro_export]
macro_rules! define_read_methods {
    ($lifetime:lifetime) => {
        fn get_user_id_by_telegram_id(&self, telegram_id: i64) -> Result<Option<trove::ObjectId>> {
            Ok(self
                .sweater_transaction
                .chest_transaction
                .select(
                    &vec![(
                        trove::IndexRecordType::Direct,
                        trove::path_segments!("telegram_id"),
                        serde_json::to_value(telegram_id)?,
                    )],
                    &vec![],
                    None,
                )?
                .next()?)
        }

        fn get_cantors_telegram_user_ids(&self) -> Result<Vec<i64>> {
            self.sweater_transaction
                .chest_transaction
                .select(
                    &vec![(
                        trove::IndexRecordType::Direct,
                        trove::path_segments!("role"),
                        serde_json::to_value(Role::Cantor)?,
                    )],
                    &vec![],
                    None,
                )?
                .map(|user_id| {
                    Ok(serde_json::from_value::<String>(
                        self.sweater_transaction
                            .chest_transaction
                            .get(&user_id, &trove::path_segments!("telegram_id"))?
                            .unwrap(),
                    )?
                    .parse::<i64>()?)
                })
                .collect()
        }
    };
}

pub trait ReadTransactionMethods<'a> {
    fn get_user_id_by_telegram_id(&self, telegram_id: i64) -> Result<Option<trove::ObjectId>>;
    fn get_cantors_telegram_user_ids(&self) -> Result<Vec<i64>>;
}

impl<'a> ReadTransactionMethods<'a> for ReadTransaction<'a> {
    define_read_methods!('a);
}
