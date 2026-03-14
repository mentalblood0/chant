pub mod read_transaction;
pub mod user;
pub mod write_transaction;

use anyhow::{anyhow, Context, Result};
use fallible_iterator::FallibleIterator;
use frankenstein::TelegramApi;
use trove::path_segments;

use crate::read_transaction::{ReadTransaction, ReadTransactionMethods};
use crate::user::{MessageGlobalId, User};
use crate::write_transaction::WriteTransaction;

wool::define_sweater!(sweater(
    users
) use {
});

#[derive(serde::Deserialize)]
pub struct ChantConfig {
    pub sweater: sweater::SweaterConfig,
    pub token: String,
    pub users: Vec<User>,
}

pub struct Chant {
    pub sweater: sweater::Sweater,
    pub bot: frankenstein::client_ureq::Bot,
    pub config: ChantConfig,
}

impl Chant {
    pub fn new(config: ChantConfig) -> Result<Self> {
        let token = config.token.clone();
        let users_to_add = config.users.clone();
        let mut result = Self {
            sweater: sweater::Sweater::new(config.sweater.clone())?,
            bot: frankenstein::client_ureq::Bot::new(&token),
            config,
        };
        result.lock_all_and_write(|transaction| transaction.add_users(&users_to_add))?;
        Ok(result)
    }

    pub fn lock_all_and_write<'a, F, R>(&'a mut self, mut f: F) -> Result<R>
    where
        F: FnMut(&mut WriteTransaction<'_, '_, '_, '_, '_>) -> Result<R>,
    {
        self.sweater
            .lock_all_and_write(|sweater_write_transaction| {
                f(&mut WriteTransaction {
                    sweater_transaction: sweater_write_transaction,
                })
            })
            .with_context(|| "Can not lock chest and initiate write transaction")
    }

    pub fn lock_all_writes_and_read<F, R>(&self, mut f: F) -> Result<R>
    where
        F: FnMut(ReadTransaction) -> Result<R>,
    {
        self.sweater
            .lock_all_writes_and_read(|sweater_read_transaction| {
                f(ReadTransaction {
                    sweater_transaction: &sweater_read_transaction,
                })
            })
            .with_context(|| {
                "Can not lock all write operations on chest and initiate read transaction"
            })
    }

    pub fn get_file_id(message: &frankenstein::types::Message) -> Option<String> {
        if let Some(ref document) = message.document {
            if let Some(ref file_name) = document.file_name {
                if file_name.ends_with(".txt") {
                    return Some(document.file_id.clone());
                }
            }
        }
        None
    }

    pub fn run(&mut self) -> Result<()> {
        let mut offset: i64 = 0;

        loop {
            let get_updates_params = frankenstein::methods::GetUpdatesParams::builder()
                .allowed_updates(vec![
                    frankenstein::types::AllowedUpdate::Message,
                    frankenstein::types::AllowedUpdate::MessageReaction,
                ])
                .offset(offset)
                .build();

            let updates = self.bot.get_updates(&get_updates_params)?;

            for update in updates.result {
                if let frankenstein::updates::UpdateContent::Message(message) = &update.content {
                    let user_id: trove::DocumentId = message.chat.id.into();
                    let mut text_option = None;
                    if let Some(ref message_text) = message.text {
                        text_option = Some(message_text.clone());
                    } else if let Some(file_id) = Self::get_file_id(message) {
                        if let Ok(file) = self.bot.get_file(
                            &frankenstein::methods::GetFileParams::builder()
                                .file_id(file_id)
                                .build(),
                        ) {
                            if let Some(file_path) = file.result.file_path {
                                let url = format!(
                                    "https://api.telegram.org/file/bot{}/{}",
                                    self.config.token, file_path
                                );
                                text_option = Some(
                                    frankenstein::ureq::get(&url)
                                        .call()?
                                        .into_body()
                                        .read_to_string()?,
                                );
                            }
                        }
                    };
                    if let Some(text) = text_option {
                        self.lock_all_and_write(|transaction| {
                            transaction.queue_commands(
                                MessageGlobalId {
                                    message_id: message.message_id,
                                    chat_id: message.chat.id,
                                },
                                user_id.clone(),
                                &text,
                            )
                        })?;
                        let sent_to_cantors_messages_ids =
                            self.lock_all_writes_and_read(|transaction| {
                                fallible_iterator::convert(
                                    transaction
                                        .get_cantors_user_ids()?
                                        .iter()
                                        .map(|cantor_user_id| Ok(cantor_user_id)),
                                )
                                .map(|cantor_user_id| {
                                    Ok(MessageGlobalId {
                                        message_id: self.forward_message(
                                            message.message_id,
                                            cantor_user_id.clone().into(),
                                        )?,
                                        chat_id: cantor_user_id.clone().into(),
                                    })
                                })
                                .collect::<Vec<_>>()
                            })?;
                        self.lock_all_and_write(|transaction| {
                            transaction
                                .sweater_transaction
                                .chest_transaction
                                .users_update(
                                    user_id.clone(),
                                    path_segments!(
                                        "commands_queue",
                                        "sent_to_cantors_messages_ids"
                                    ),
                                    serde_json::to_value(&sent_to_cantors_messages_ids)?,
                                )?;
                            Ok(())
                        })?;
                        self.set_reaction(
                            &MessageGlobalId {
                                message_id: message.message_id,
                                chat_id: message.chat.id,
                            },
                            "✍️",
                        )?;
                    }
                }
                if let frankenstein::updates::UpdateContent::MessageReaction(reaction) =
                    &update.content
                {
                    for reaction_type in &reaction.new_reaction {
                        if let frankenstein::types::ReactionType::Emoji(emoji) = reaction_type {
                            if emoji.emoji == "👍" {
                                let approved_queued_commands =
                                    self.lock_all_and_write(|transaction| {
                                        let user_which_commands_were_approved = transaction
                                            .sweater_transaction
                                            .chest_transaction
                                            .users_select(
                                                &vec![(
                                                    trove::search_path_segments!(
                                                        "commands_queue",
                                                        "sent_to_cantors_messages_ids",
                                                        ()
                                                    ),
                                                    reaction.chat.id.into(),
                                                )],
                                                &vec![],
                                                None,
                                            )?
                                            .next()?
                                            .ok_or_else(|| {
                                                anyhow!("Can not find user with source message")
                                            })?;
                                        let approved_queued_commands = serde_json::from_value::<
                                            Option<user::QueuedCommands>,
                                        >(
                                            transaction
                                                .sweater_transaction
                                                .chest_transaction
                                                .users_get(
                                                    &user_which_commands_were_approved,
                                                    &path_segments!("commands_queue",),
                                                )?
                                                .ok_or_else(|| {
                                                    anyhow!("Can not get commands queue")
                                                })?,
                                        )
                                        .with_context(|| "Can not parse commands queue from JSON")?
                                        .ok_or_else(|| {
                                            anyhow!("Expected queued commands but there is none")
                                        })?;
                                        transaction
                                            .sweater_transaction
                                            .chest_transaction
                                            .users_remove(
                                                &user_which_commands_were_approved,
                                                &path_segments!("commands_queue"),
                                            )?;
                                        for command in approved_queued_commands.commands.iter() {
                                            transaction
                                                .sweater_transaction
                                                .execute_command(&command)?;
                                        }
                                        Ok(approved_queued_commands)
                                    })?;
                                self.set_reaction(
                                    &approved_queued_commands.source_message_global_id,
                                    "👍",
                                )?;
                                for sent_to_cantor_global_message_id in
                                    approved_queued_commands.sent_to_cantors_messages_ids
                                {
                                    self.bot.delete_message(
                                        &frankenstein::methods::DeleteMessageParams::builder()
                                            .chat_id(sent_to_cantor_global_message_id.chat_id)
                                            .message_id(sent_to_cantor_global_message_id.message_id)
                                            .build(),
                                    )?;
                                }
                            }
                        }
                    }
                }
                offset = update.update_id as i64 + 1;
            }
        }
    }

    pub fn set_reaction(
        &self,
        message_global_id: &MessageGlobalId,
        reaction_emoji_str: &str,
    ) -> Result<()> {
        self.bot.set_message_reaction(
            &frankenstein::methods::SetMessageReactionParams::builder()
                .chat_id(message_global_id.chat_id)
                .message_id(message_global_id.message_id)
                .reaction(vec![frankenstein::types::ReactionType::Emoji(
                    frankenstein::types::ReactionTypeEmoji::builder()
                        .emoji(reaction_emoji_str.to_string())
                        .build(),
                )])
                .build(),
        )?;
        Ok(())
    }

    pub fn forward_message(&self, message_id: i32, to_user_id: i64) -> Result<i32> {
        Ok(self
            .bot
            .forward_message(
                &frankenstein::methods::ForwardMessageParams::builder()
                    .chat_id(to_user_id)
                    .from_chat_id(to_user_id)
                    .message_id(message_id)
                    .build(),
            )?
            .result
            .message_id)
    }
}

fn main() -> Result<()> {
    let config_path = std::env::args()
        .nth(1)
        .context("Usage: chant <config_path>")?;

    let mut chant = Chant::new(
        serde_saphyr::from_str(
            &std::fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to open config file: {}", config_path))?,
        )
        .with_context(|| format!("Failed to parse config file: {}", config_path))?,
    )?;

    chant.run()
}
