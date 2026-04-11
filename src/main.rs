pub mod commands;
pub mod read_transaction;
pub mod user;
pub mod write_transaction;

use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use fallible_iterator::FallibleIterator;
use frankenstein::TelegramApi;
use trove::path_segments;

use crate::read_transaction::{ReadTransaction, ReadTransactionMethods};
use crate::user::{MessageGlobalId, Role, User};
use crate::write_transaction::WriteTransaction;

wool::define_sweater!(sweater(
    users
) use {
});

#[derive(serde::Deserialize)]
pub struct Bounds {
    pub min: u16,
    pub max: u16,
}

#[derive(serde::Deserialize)]
pub struct BatchLimits {
    pub bytes: Bounds,
}

#[derive(serde::Deserialize)]
pub struct Limits {
    pub batch: BatchLimits,
}

#[derive(serde::Deserialize)]
pub struct ChantConfig {
    pub sweater: sweater::SweaterConfig,
    pub token: String,
    pub users: Vec<User>,
    pub limits: Limits,
    pub graph_file_path: PathBuf,
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
        let graph_definition = result.lock_all_and_write(|transaction| {
            transaction.add_users(&users_to_add)?;
            transaction.get_graph_definition()
        })?;
        result.update_graph_file(&graph_definition)?;
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
            .context("Can not lock chest and initiate write transaction")
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
            .context("Can not lock all write operations on chest and initiate read transaction")
    }

    pub fn update_graph_file(&self, graph_definition: &String) -> Result<()> {
        let mut command = std::process::Command::new("dot")
            .args([
                "-Tsvg",
                &format!(
                    "-o{}",
                    self.config
                        .graph_file_path
                        .clone()
                        .to_str()
                        .with_context(|| format!(
                            "Can not use graph file path {:?} as it is invalid",
                            self.config.graph_file_path
                        ))?
                ),
            ])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .context("Can not spawn command to generate graph from graph definition")?;
        command
            .stdin
            .take()
            .context(
                "Can not own standard input handle of command spawned to generate grpah from \
                 graph definition",
            )?
            .write_all(graph_definition.as_bytes())
            .context("Can not write graph definition to standard input of command")?;
        let command_result = command
            .wait_with_output()
            .context("Can not execute command to generate graph from graph definition")?;
        if !command_result.status.success() {
            Err(anyhow!(
                "Error while executing command to generate graph from graph definition:\n{}",
                String::from_utf8(command_result.stderr)?
            ))
        } else {
            Ok(())
        }
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

    pub fn process_message_document(
        &mut self,
        message: &frankenstein::types::Message,
    ) -> Result<()> {
        if let Some(ref document) = message.document {
            if let Some(file_size) = document.file_size {
                if file_size > self.config.limits.batch.bytes.max as u64 {
                    return Result::Err(anyhow!(
                        "Can not process document file with size {file_size} bytes > {} bytes",
                        self.config.limits.batch.bytes.max
                    ));
                }
                if file_size < self.config.limits.batch.bytes.min as u64 {
                    return Result::Err(anyhow!(
                        "Can not process document file with size {file_size} bytes < {} bytes",
                        self.config.limits.batch.bytes.min
                    ));
                }
            }
            if let Some(ref file_name) = document.file_name {
                if file_name.ends_with(".txt") {
                    let file_id = &document.file_id;
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
                            let text = frankenstein::ureq::get(&url)
                                .call()?
                                .into_body()
                                .read_to_string()?;
                            if let Err(error_queuing_commands) =
                                self.lock_all_and_write(|transaction| {
                                    transaction.queue_commands(
                                        MessageGlobalId {
                                            message_id: message.message_id,
                                            chat_id: message.chat.id,
                                        },
                                        message.chat.id.into(),
                                        &text,
                                    )
                                })
                            {
                                return Result::Err(anyhow!(
                                    "There was error queuing commands: {}",
                                    error_queuing_commands
                                ));
                            } else {
                                let sent_to_cantors_messages_ids = self
                                        .lock_all_writes_and_read(|transaction| {
                                            let mut result = vec![];
                                            for cantor_user_id in transaction.get_cantors_user_ids()? {
                                                match self.bot.forward_message(
                                                    &frankenstein::methods::ForwardMessageParams::builder()
                                                        .chat_id(<trove::DocumentId as Into<i64>>::into(cantor_user_id.clone()))
                                                        .from_chat_id(message.chat.id)
                                                        .message_id(message.message_id)
                                                        .build(),
                                                ) {
                                                    Ok(message_forwarding_result) => {
                                                        result.push(MessageGlobalId {
                                                            message_id: message_forwarding_result.result.message_id,
                                                            chat_id: cantor_user_id.clone().into(),
                                                        });
                                                    }
                                                    Err(error) => {
                                                        if !error.to_string().contains("chat not found") {
                                                            return Err(anyhow!(error));
                                                        }
                                                    }
                                                }
                                            }
                                            Ok(result)
                                        })?;
                                self.lock_all_and_write(|transaction| {
                                    transaction
                                        .sweater_transaction
                                        .chest_transaction
                                        .users_set(
                                            message.chat.id.into(),
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
                    }
                }
            }
        }
        Ok(())
    }

    pub fn process_message_text(
        &mut self,
        message: &frankenstein::types::Message,
        user_role: &Role,
    ) -> Result<()> {
        if let Some(ref message_text) = message.text {
            if message_text == "/graph" {
                self.bot.send_document(
                    &frankenstein::methods::SendDocumentParams::builder()
                        .chat_id(message.chat.id)
                        .document(frankenstein::input_file::InputFile {
                            path: self.config.graph_file_path.clone(),
                        })
                        .reply_parameters(
                            frankenstein::types::ReplyParameters::builder()
                                .message_id(message.message_id)
                                .build(),
                        )
                        .build(),
                )?;
            } else {
                let reply_text = self
                    .lock_all_and_write(|transaction| {
                        let command = commands::Command::from_text(
                            message_text,
                            &sweater::AliasesResolver {
                                read_able_transaction: transaction.sweater_transaction,
                                known_aliases: BTreeMap::new(),
                            },
                        )?;
                        if !command.is_allowed_for(user_role) {
                            return Err(anyhow!(
                                "Execution of command {command:?} not allowed for user with role \
                                 {user_role:?}"
                            ));
                        };
                        transaction.execute_command(&command)
                    })
                    .context("Error parsing and executing commands")?;
                if reply_text.is_empty() {
                    self.set_reaction(
                        &MessageGlobalId {
                            message_id: message.message_id,
                            chat_id: message.chat.id,
                        },
                        "🤷",
                    )?;
                } else {
                    self.bot.send_message(
                        &frankenstein::methods::SendMessageParams::builder()
                            .parse_mode(frankenstein::ParseMode::MarkdownV2)
                            .chat_id(message.chat.id)
                            .reply_parameters(
                                frankenstein::types::ReplyParameters::builder()
                                    .message_id(message.message_id)
                                    .build(),
                            )
                            .link_preview_options(
                                frankenstein::types::LinkPreviewOptions::builder()
                                    .is_disabled(true)
                                    .build(),
                            )
                            .text(reply_text)
                            .build(),
                    )?;
                }
            }
        }
        Ok(())
    }

    pub fn reply_with_error_text_if_error<T>(
        &self,
        message: &frankenstein::types::Message,
        message_processing_result: Result<T>,
    ) -> Result<()>
    where
        T: std::fmt::Debug,
    {
        if let Err(error) = message_processing_result {
            self.set_reaction(
                &MessageGlobalId {
                    message_id: message.message_id,
                    chat_id: message.chat.id,
                },
                "🤔",
            )?;
            self.bot.send_message(
                &frankenstein::methods::SendMessageParams::builder()
                    .chat_id(message.chat.id)
                    .reply_parameters(
                        frankenstein::types::ReplyParameters::builder()
                            .message_id(message.message_id)
                            .build(),
                    )
                    .text(format!(
                        "There was a error while processing your message: {error:?}"
                    ))
                    .build(),
            )?;
        }
        Ok(())
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
                // offset = update.update_id as i64 + 1;
                // continue;
                if let frankenstein::updates::UpdateContent::Message(message) = &update.content {
                    if let Some(message_user_json_value) =
                        self.lock_all_writes_and_read(|transaction| {
                            transaction
                                .sweater_transaction
                                .chest_transaction
                                .users_get(&message.chat.id.into(), &vec![])
                        })?
                    {
                        let message_user = serde_json::from_value::<User>(message_user_json_value)?;
                        {
                            let message_document_processing_result = self
                                .process_message_document(message)
                                .context("Can not process message document");
                            self.reply_with_error_text_if_error(
                                message,
                                message_document_processing_result,
                            )?;
                        }
                        {
                            let message_text_processing_result = self
                                .process_message_text(message, &message_user.role)
                                .context("Can not process message text");
                            self.reply_with_error_text_if_error(
                                message,
                                message_text_processing_result,
                            )?;
                        }
                    } else {
                        continue;
                    }
                }
                if let frankenstein::updates::UpdateContent::MessageReaction(reaction) =
                    &update.content
                {
                    for reaction_type in &reaction.new_reaction {
                        if let frankenstein::types::ReactionType::Emoji(emoji) = reaction_type {
                            if emoji.emoji == "👍" || emoji.emoji == "👎" {
                                let (
                                    approved,
                                    source_message_global_id,
                                    sent_to_cantors_global_messages_ids,
                                    commands_execution_error_option,
                                    graph_definition,
                                ) = self.lock_all_and_write(|transaction| {
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
                                                serde_json::to_value(MessageGlobalId {
                                                    chat_id: reaction.chat.id.into(),
                                                    message_id: reaction.message_id,
                                                })?,
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
                                            .ok_or_else(|| anyhow!("Can not get commands queue"))?,
                                    )
                                    .context("Can not parse commands queue from JSON")?
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
                                    let mut commands_execution_error_option = None;
                                    let approved = emoji.emoji == "👍";
                                    if approved {
                                        for command in approved_queued_commands.commands.iter() {
                                            if let Err(commands_execution_error) = transaction
                                                .sweater_transaction
                                                .execute_command(&command)
                                            {
                                                commands_execution_error_option =
                                                    Some(commands_execution_error);
                                                break;
                                            }
                                        }
                                    }
                                    Ok((
                                        approved,
                                        approved_queued_commands.source_message_global_id,
                                        approved_queued_commands.sent_to_cantors_messages_ids,
                                        commands_execution_error_option,
                                        transaction.get_graph_definition()?,
                                    ))
                                })?;
                                if let Some(commands_execution_error) =
                                    commands_execution_error_option
                                {
                                    self.set_reaction(&source_message_global_id, "🤔")?;
                                    self.bot.send_message(
                                        &frankenstein::methods::SendMessageParams::builder()
                                            .chat_id(source_message_global_id.chat_id)
                                            .reply_parameters(
                                                frankenstein::types::ReplyParameters::builder()
                                                    .message_id(source_message_global_id.message_id)
                                                    .build(),
                                            )
                                            .text(format!(
                                                "There was error executing commands: {}",
                                                commands_execution_error
                                            ))
                                            .build(),
                                    )?;
                                } else {
                                    self.set_reaction(&source_message_global_id, &emoji.emoji)?;
                                }
                                for sent_to_cantor_global_message_id in
                                    sent_to_cantors_global_messages_ids
                                {
                                    self.bot.delete_message(
                                        &frankenstein::methods::DeleteMessageParams::builder()
                                            .chat_id(sent_to_cantor_global_message_id.chat_id)
                                            .message_id(sent_to_cantor_global_message_id.message_id)
                                            .build(),
                                    )?;
                                }
                                if approved {
                                    self.update_graph_file(&graph_definition)?;
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
}

fn main() -> Result<()> {
    Chant::new(serde_saphyr::from_reader(std::io::stdin()).context("Can not parse configuration")?)?
        .run()
}
