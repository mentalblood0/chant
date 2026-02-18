use anyhow::{Context, Result};
use fallible_iterator::FallibleIterator;
use frankenstein::TelegramApi;
use std::collections::BTreeMap;
use woollib::commands::CommandsIterator;
use woollib::sweater::{Sweater, SweaterConfig};

#[derive(serde::Deserialize)]
struct ChantConfig {
    sweater: SweaterConfig,
    token: String,
}

struct Chant {
    sweater: Sweater,
    bot: frankenstein::client_ureq::Bot,
    token: String,
}

impl Chant {
    fn new(config: ChantConfig) -> Result<Self> {
        let token = config.token.clone();
        Ok(Self {
            sweater: Sweater::new(config.sweater)?,
            bot: frankenstein::client_ureq::Bot::new(&token),
            token,
        })
    }

    fn get_file_id(message: &frankenstein::types::Message) -> Option<String> {
        if let Some(ref document) = message.document {
            if let Some(ref file_name) = document.file_name {
                if file_name.ends_with(".txt") {
                    return Some(document.file_id.clone());
                }
            }
        }
        None
    }

    fn run(&mut self) -> Result<()> {
        let mut offset: i64 = 0;

        loop {
            let get_updates_params = frankenstein::methods::GetUpdatesParams::builder()
                .allowed_updates(vec![frankenstein::types::AllowedUpdate::Message])
                .offset(offset)
                .build();

            let updates = self.bot.get_updates(&get_updates_params)?;

            for update in updates.result {
                if let frankenstein::updates::UpdateContent::Message(message) = &update.content {
                    if let Some(file_id) = Self::get_file_id(message) {
                        if let Ok(file) = self.bot.get_file(
                            &frankenstein::methods::GetFileParams::builder()
                                .file_id(file_id)
                                .build(),
                        ) {
                            if let Some(file_path) = file.result.file_path {
                                let url = format!(
                                    "https://api.telegram.org/file/bot{}/{}",
                                    self.token, file_path
                                );
                                let file_text = frankenstein::ureq::get(&url)
                                    .call()?
                                    .into_body()
                                    .read_to_string()?;
                                let mut commands = vec![];
                                self.sweater.lock_all_writes_and_read(|transaction| {
                                    commands = CommandsIterator::new(
                                        &file_text,
                                        &transaction.sweater_config.supported_relations_kinds,
                                        &mut woollib::aliases_resolver::AliasesResolver {
                                            read_able_transaction: &transaction,
                                            known_aliases: BTreeMap::new(),
                                        },
                                    )
                                    .collect::<Vec<_>>()?;
                                    Ok(())
                                })?;
                                self.sweater.lock_all_and_write(|transaction| {
                                    for command in &commands {
                                        println!("executing {command:?}");
                                        transaction.execute_command(&command)?;
                                    }
                                    Ok(())
                                })?;

                                self.set_reaction(message, "✍️")?;
                            }
                        }
                    }
                }
                offset = update.update_id as i64 + 1;
            }
        }
    }

    fn set_reaction(
        &self,
        message: &frankenstein::types::Message,
        reaction_emoji_str: &str,
    ) -> Result<()> {
        let params = frankenstein::methods::SetMessageReactionParams::builder()
            .chat_id(message.chat.id)
            .message_id(message.message_id)
            .reaction(vec![frankenstein::types::ReactionType::Emoji(
                frankenstein::types::ReactionTypeEmoji::builder()
                    .emoji(reaction_emoji_str.to_string())
                    .build(),
            )])
            .build();

        self.bot.set_message_reaction(&params)?;
        Ok(())
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
