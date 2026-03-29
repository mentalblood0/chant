use anyhow::{anyhow, Context, Error, Result};
use fallible_iterator::FallibleIterator;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::sweater;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Command {
    GetTheses(Vec<trove::DocumentId>),
}

impl Command {
    pub fn validated(&self) -> Result<&Self> {
        match self {
            Command::GetTheses(_) => {}
        }
        Ok(self)
    }
}

pub struct CommandsIterator<'a> {
    paragraphs_iterator: Box<dyn FallibleIterator<Item = (usize, &'a str), Error = Error> + 'a>,
    aliases_resolver: &'a mut sweater::AliasesResolver<'a>,
}

impl<'a> CommandsIterator<'a> {
    pub fn new(input: &'a str, aliases_resolver: &'a mut sweater::AliasesResolver<'a>) -> Self {
        static COMMANDS_SPLIT_REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
        let commands_split_regex = COMMANDS_SPLIT_REGEX.get_or_init(|| {
            Regex::new(r#"(\r?\n|\r){2,}"#)
                .with_context(|| "Can not compile regular expression for commands splitting")
                .unwrap()
        });
        Self {
            aliases_resolver: aliases_resolver,
            paragraphs_iterator: Box::new(fallible_iterator::convert(
                commands_split_regex
                    .split(input)
                    .map(|paragraph| paragraph.trim())
                    .filter(|paragraph| !paragraph.is_empty())
                    .enumerate()
                    .map(|index_and_paragraph| Ok(index_and_paragraph)),
            )),
        }
    }
}

impl<'a> FallibleIterator for CommandsIterator<'a> {
    type Item = Command;
    type Error = Error;

    fn next(&mut self) -> Result<Option<Self::Item>> {
        if let Some((paragraph_index, paragraph)) = self.paragraphs_iterator.next()? {
            let lines = paragraph.split('\n').collect::<Vec<_>>();
            static COMMAND_FIRST_LINE_REGEX: std::sync::OnceLock<Regex> =
                std::sync::OnceLock::new();
            let command_first_line_regex = COMMAND_FIRST_LINE_REGEX.get_or_init(|| {
                Regex::new(r#"^ *(\?) *$"#)
                    .with_context(|| {
                        "Can not compile regular expression for parsing first line of command"
                    })
                    .unwrap()
            });
            if let Some(captures) = command_first_line_regex.captures(lines[0]) {
                let operation_char = captures[1].chars().next().unwrap();
                Ok(Some(
                    match (operation_char, lines.len()) {
                        ('?', 2..) => Command::GetTheses(
                            fallible_iterator::convert(
                                lines[1..].iter().map(|line| sweater::Reference::new(line)),
                            )
                            .map(|reference| {
                                self.aliases_resolver.get_thesis_id_by_reference(&reference)
                            })
                            .collect()
                            .with_context(|| {
                                format!(
                                    "Can not parse {}-th paragraph {paragraph:?}",
                                    paragraph_index + 1
                                )
                            })?,
                        ),
                        _ => {
                            return Err(anyhow!(
                                "Unsupported operation character and lines count combination \
                                 ({:?}, {}) in first line {:?} of {}-th paragraph {:?}, supported \
                                 combinations are ('?', 2..) for getting theses by references",
                                operation_char,
                                lines.len(),
                                lines[0],
                                paragraph_index + 1,
                                paragraph
                            ));
                        }
                    }
                    .validated()
                    .with_context(|| {
                        format!(
                            "Invalid command parsed from {}-th paragraph {:?}",
                            paragraph_index + 1,
                            paragraph
                        )
                    })?
                    .to_owned(),
                ))
            } else {
                Err(anyhow!(
                    "Can not parse first line {:?} in {}-th paragraph {:?}",
                    lines[0],
                    paragraph_index + 1,
                    paragraph
                ))
            }
        } else {
            Ok(None)
        }
    }
}
