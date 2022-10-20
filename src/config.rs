use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use clap::{ArgAction, Parser};
use clap_verbosity_flag::Verbosity;

// Fish -> Nushell completion conversion script options
#[derive(Debug, Parser)]
#[command(version)]
pub struct Config {
    #[clap(flatten)]
    pub verbose: Verbosity,
    /// Where converted completion files will be stored
    #[arg(short, long, default_value_os_t = PathBuf::from(env::var("HOME").expect("$HOME is not set")).join(".config/nushell/completions/definitions"))]
    pub output_dir: PathBuf,
    /// Directory containing patch files to change the generated completions
    #[arg(short, long, default_value_os_t = PathBuf::from(env::var("HOME").expect("$HOME is not set")).join(".config/nushell/completions/patches"))]
    pub patch_dir: PathBuf,
    /// The original fish completion files to be converted
    pub sources: Vec<OsString>,
    #[arg(long = "no-parse", action = ArgAction::SetFalse, default_value_t = true)]
    pub parse: bool,
    #[arg(long = "no-convert", action = ArgAction::SetFalse, default_value_t = true)]
    pub convert: bool,
}

pub static CONFIG: LazyLock<Config> = LazyLock::new(Config::parse);

impl Config {
    pub(crate) fn verbose() -> &'static Verbosity {
        &CONFIG.verbose
    }
    pub(crate) fn output_dir() -> &'static Path {
        CONFIG.output_dir.as_path()
    }
    pub(crate) fn sources() -> &'static Vec<OsString> {
        &CONFIG.sources
    }
    pub(crate) fn patch_dir() -> &'static Path {
        CONFIG.patch_dir.as_path()
    }
    pub(crate) fn patch() -> bool {
        CONFIG.parse
    }
    pub(crate) fn convert() -> bool {
        CONFIG.convert
    }
}
