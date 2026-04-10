use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

use crate::sweater::{self, AliasesResolver};
use crate::user::{Role, User};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Command {
    GetThesisByReference(trove::DocumentId),
    GetThesesByTags(Vec<sweater::Tag>),
    AddOfferers(Vec<User>),
    PromoteToCantor(trove::DocumentId),
}

pub trait RoleRestricted {
    fn is_allowed_for(&self, role: &Role) -> bool;
}

impl Command {
    pub fn validated(&self) -> Result<&Self> {
        match self {
            Command::GetThesisByReference(_) => {}
            Command::GetThesesByTags(tags) => {
                for tag in tags {
                    tag.validated()?;
                }
            }
            Command::AddOfferers(_) => {}
            Command::PromoteToCantor(_) => {}
        }
        Ok(self)
    }

    pub fn is_allowed_for(&self, role: &Role) -> bool {
        match (role, self) {
            (Role::Offerer, Command::GetThesisByReference(_)) => true,
            (Role::Offerer, Command::GetThesesByTags(_)) => true,
            (Role::Offerer, Command::AddOfferers(_)) => false,
            (Role::Offerer, Command::PromoteToCantor(_)) => false,

            (Role::Cantor, Command::GetThesisByReference(_)) => true,
            (Role::Cantor, Command::GetThesesByTags(_)) => true,
            (Role::Cantor, Command::AddOfferers(_)) => true,
            (Role::Cantor, Command::PromoteToCantor(_)) => true,
        }
    }

    pub fn from_text(text: &String, aliases_resolver: &AliasesResolver) -> Result<Command> {
        let command_text_splitted = text.split(' ').collect::<Vec<_>>();
        Self::from_splitted_text(command_text_splitted, aliases_resolver)
    }

    pub fn from_splitted_text(
        command_text_splitted: Vec<&str>,
        aliases_resolver: &AliasesResolver,
    ) -> Result<Command> {
        let command_name = command_text_splitted
            .get(0)
            .ok_or(anyhow!("Can not parse empty command"))?;
        let command_arguments = command_text_splitted[1..].to_vec();
        Ok(match (*command_name, command_arguments.len()) {
            ("/start", 1) => Self::from_splitted_text(
                format!("/{}", command_arguments[0])
                    .splitn(2, '_')
                    .collect(),
                aliases_resolver,
            )?,
            ("/reference", 1) => {
                let argument = command_arguments[0];
                Command::GetThesisByReference(aliases_resolver.get_thesis_id_by_reference(
                    &sweater::Reference::new(argument).with_context(|| {
                        anyhow!(
                            "Can not parse /reference command because argument {argument:?} is \
                             invalid reference"
                        )
                    })?,
                )?)
            }
            ("/tags", 1..) => Command::GetThesesByTags(
                command_arguments[1..]
                    .iter()
                    .map(|line| sweater::Tag(line.to_string()))
                    .collect(),
            ),
            ("/add_offerers", 1..) => {
                let mut result = vec![];
                for argument in command_arguments[1..].iter() {
                    result.push(User {
                        telegram_id: argument.parse::<i64>()?,
                        role: Role::Offerer,
                        commands_queue: None,
                    });
                }
                Command::AddOfferers(result)
            }
            ("/promote_to_cantor", 1) => {
                let argument = command_arguments[0];
                Command::PromoteToCantor(
                    argument
                        .parse::<i64>()
                        .with_context(|| {
                            format!(
                                "Can not parse command argument {argument:?} as user telegram id"
                            )
                        })?
                        .into(),
                )
            }
            _ => {
                return Err(anyhow!(
                    "Can not parse command: unsupported command name ({command_name:?}) or amount \
                     of arguments ({}). Supported commands are:\n/reference \
                     one_reference_to_search_by\n/tags one or more some tags to search \
                     by\n/add_offerers one or more users' telegram \
                     identifiers\n/promote_to_cantor one_offerer_telegram_identifier",
                    command_arguments.len()
                ));
            }
        }
        .validated()
        .context("Invalid command parsed")?
        .to_owned())
    }
}
