use std::borrow::Cow;

use clap::Parser;
use lazy_regex::regex_replace_all;
use serde::Serialize;

/// A Fish completion definition
#[derive(Debug, Parser, Clone, Default, Serialize)]
pub(crate) struct CompletionLine {
    /// Short options (-e -g)
    #[arg(short, long)]
    pub(crate) short: Vec<String>,
    /// Long options (--for --example)
    #[arg(short, long)]
    pub(crate) long: Vec<String>,
    #[arg(short, long)]
    pub(crate) command: Option<String>,
    /// An unprefixed positional argument
    ///
    /// TODO: I think this needs implemented.
    #[arg(short, long)]
    pub(crate) argument: Option<String>,
    #[arg(short, long)]
    pub(crate) description: Option<String>,
    /// Old-style long options (-like -this)
    #[arg(short, long)]
    pub(crate) old_option: Vec<String>,
}

impl CompletionLine {
    /// Fish is somehow totally cool parsing a completion like
    ///
    /// ```fish
    /// complete -s r -d "--recursive alias"
    /// ```
    /// whereas clap barfs on that. So, here we insert a filler character
    /// beginning of an argument with a - so that the argument can be passed
    /// to clap.
    pub(crate) fn escape_options_which_start_with_a_dash(line: &str) -> Cow<'_, str> {
        regex_replace_all!(r#" \-([slado]) (["']?)\-"#, line, |_, opt, quote| format!(
            " -{opt} {quote}\u{FFFC}-"
        ))
    }
    /// Undo the effects of [`CompletionLine::escape_options_which_start_with_a_dash()`].
    pub(crate) fn unescape_option_which_starts_with_a_dash(option: impl AsRef<str>) -> String {
        option.as_ref().replace('\u{FFFC}', "")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static CASES: [(&str, &str); 2] = [
        (" -d '-test'", " -d '\u{FFFC}-test'"),
        (
            " -l ---test -d '-with multiple opts' -s -",
            " -l \u{FFFC}---test -d '\u{FFFC}-with multiple opts' -s \u{FFFC}-",
        ),
    ];

    #[test]
    fn test_escape_options_which_start_with_a_dash() {
        for (before, after) in CASES {
            assert_eq!(
                CompletionLine::escape_options_which_start_with_a_dash(before),
                after
            );
        }
    }

    #[test]
    fn test_unescape_options_which_start_with_a_dash() {
        assert_eq!(
            CompletionLine::unescape_option_which_starts_with_a_dash("\u{FFFC}-test"),
            "-test"
        );
    }
}
