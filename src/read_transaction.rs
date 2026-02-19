pub struct ReadTransaction<'a> {
    pub sweater_transaction: &'a woollib::read_transaction::ReadTransaction<'a>,
}

#[macro_export]
macro_rules! define_read_methods {
    ($lifetime:lifetime) => {};
}

pub trait ReadTransactionMethods<'a> {}

impl<'a> ReadTransactionMethods<'a> for ReadTransaction<'a> {
    define_read_methods!('a);
}
