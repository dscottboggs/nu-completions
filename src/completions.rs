use anyhow::anyhow;
use clap::Parser;
use derive_deref::Deref;
use log::{as_serde, error, warn};
use std::sync::{Arc, RwLock};
use tokio::task::JoinSet;

use defaultmap::DefaultHashMap;

use crate::completion_line::CompletionLine;

#[derive(Debug, Default, Deref, Clone)]
pub(crate) struct Completions(Arc<RwLock<DefaultHashMap<String, Vec<CompletionLine>>>>);

impl Completions {
    pub(crate) async fn parse(
        lines: impl Iterator<Item = impl AsRef<str>>,
    ) -> anyhow::Result<Self> {
        Completions::default().parse_completions(lines).await
    }
    pub(crate) async fn parse_one_completion(
        self,
        completion: impl AsRef<str>,
    ) -> anyhow::Result<()> {
        let completion_ref = completion.as_ref();
        let completion = CompletionLine::escape_options_which_start_with_a_dash(completion_ref);
        let args = shell_words::split(&completion.replace("\\'", "'\"'\"'")).map_err(|err| {
            warn!(line = completion; "error parsing shell words");
            err
        })?;
        let mut completion = CompletionLine::try_parse_from(args).map_err(|err| {
            warn!(line = completion; "error parsing completion line");
            err
        })?;
        let Some(command_name) = &completion.command else {
            warn!(completion = as_serde!(completion), line_text = completion_ref; "completion contained no command name");
            return Err(anyhow!("completion contained no command name: {completion:?}"));
        };
        completion.description = completion
            .description
            .map(CompletionLine::unescape_option_which_starts_with_a_dash);
        completion.short = completion
            .short
            .iter()
            .map(CompletionLine::unescape_option_which_starts_with_a_dash)
            .collect();
        completion.long = completion
            .long
            .iter()
            .map(CompletionLine::unescape_option_which_starts_with_a_dash)
            .collect();
        completion.argument = completion
            .argument
            .map(CompletionLine::unescape_option_which_starts_with_a_dash);
        completion.old_option = completion
            .old_option
            .iter()
            .map(CompletionLine::unescape_option_which_starts_with_a_dash)
            .collect();

        self.0
            .write()
            .expect("poisoned mutex")
            .get_mut(command_name.to_string())
            .push(completion);
        Ok(())
    }

    pub(crate) async fn parse_completions(
        self,
        lines: impl Iterator<Item = impl AsRef<str>>,
    ) -> anyhow::Result<Self> {
        let mut tasks = JoinSet::new();
        for line in lines {
            let it = self.clone();
            let line: String = line.as_ref().to_string();
            let line = if let Some(_line) = line.strip_prefix(char::is_whitespace) {
                _line.to_string()
            } else {
                line
            };
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            tasks.spawn(async move { it.parse_one_completion(line).await });
        }
        while let Some(result) = tasks.join_next().await {
            let result = result?;
            if let Err(err) = result {
                error!("{err:?}");
            }
        }
        Ok(self)
    }
}

#[cfg(test)]
mod test {
    use super::Completions;
    use anyhow::anyhow;

    #[tokio::test]
    async fn test_parse_one_completion() -> anyhow::Result<()> {
        let completions = Completions::default();
        completions
            .clone()
            .parse_one_completion("complete -c mockery -s b -d 'test description'")
            .await?;
        let completion = &completions.0.read().expect("poisoned Arc")[String::from("mockery")];
        let completion = &completion[0];
        assert_eq!(
            completion
                .command
                .as_ref()
                .ok_or_else(|| anyhow!("command was None"))?
                .as_str(),
            "mockery"
        );
        assert_eq!(completion.short, vec!["b"]);
        assert_eq!(
            completion
                .description
                .as_ref()
                .ok_or_else(|| anyhow!("description was None"))?
                .as_str(),
            "test description"
        );
        assert!(completion.long.is_empty());
        assert_eq!(completion.argument, None);
        assert!(completion.old_option.is_empty());
        Ok(())
    }
}
