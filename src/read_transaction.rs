use anyhow::Result;

pub struct ReadTransaction<'a> {
    pub sweater_transaction: &'a woollib::read_transaction::ReadTransaction<'a>,
}

#[macro_export]
macro_rules! define_read_methods {
    ($lifetime:lifetime) => {
        fn get_user_id_by_telegram_id(&self, telegram_id: &str) -> Result<Option<trove::ObjectId>> {
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
    };
}

pub trait ReadTransactionMethods<'a> {
    fn get_user_id_by_telegram_id(&self, telegram_id: &str) -> Result<Option<trove::ObjectId>>;
}

impl<'a> ReadTransactionMethods<'a> for ReadTransaction<'a> {
    define_read_methods!('a);
}
