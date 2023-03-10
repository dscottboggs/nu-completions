//! This configures how the script is run through its command-line arguments.
use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use clap::{ArgAction, Args, Parser, Subcommand};
use clap_verbosity_flag::Verbosity;

// Fish -> Nushell completion conversion script options
#[derive(Debug, Parser)]
#[command(version)]
pub struct Config {
    #[clap(flatten)]
    pub verbose: Verbosity,
    /// Where converted completion files will be stored
    #[arg(
        short, long,
        default_value_os_t = xdg_config_path("nushell/completions/definitions")
    )]
    pub output_dir: PathBuf,
    /// Directory containing patch files to change the generated completions
    #[arg(
        short, long,
        default_value_os_t = xdg_config_path("nushell/completions/patches")
    )]
    pub patch_dir: PathBuf,
    /// The original fish completion files to be converted
    pub sources: Vec<OsString>,
    #[arg(
        long = "no-patch",
        action = ArgAction::SetFalse,
        default_value_t = true,
        help = "disable patching phase"
    )]
    pub patch: bool,
    #[arg(
        long = "no-convert",
        action = ArgAction::SetFalse,
        default_value_t = true,
        help = "disable conversion phase"
    )]
    pub convert: bool,
    #[arg(
        long = "no-update-patches",
        action = ArgAction::SetFalse,
        default_value_t = true,
        help = "skip updating the standard patch-set from github"
    )]
    pub update_patch_set: bool,
    #[command(subcommand)]
    pub patches: Option<PatchesCommand>,
    /// Add a line to the env config importing all of the defined completions
    #[arg(short, long)]
    pub install: bool,
    /// Where nushell's env.nu is located.
    #[arg(long, default_value_os_t = xdg_config_path("nushell/env.nu"))]
    pub install_location: PathBuf,
    #[arg(long)]
    pub fail_fast: bool,
}

#[derive(Debug, Subcommand)]
pub enum PatchesCommand {
    /// Commands related to patches
    Patches(PatchesSubCommand),
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Args, Debug)]
pub struct PatchesSubCommand {
    #[command(subcommand)]
    action: PatchesSubCommandAction,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Subcommand, Debug)]
pub enum PatchesSubCommandAction {
    /// Generate patch files from changes.
    Generate(PatchesGenerateOptions),
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Args, Debug)]
pub struct PatchesGenerateOptions {
    /// The now-modified completion definitions
    #[arg(short, long, default_value_os_t = xdg_config_path("nushell/completions/definitions"))]
    pub from: PathBuf,
    /// The folder where patch files should be placed. Existing files WILL be
    /// clobbered!
    #[arg(short, long, default_value_os_t = xdg_config_path("nushell/completions/patches"))]
    pub to: PathBuf,
    /// The original fish completion files to be converted
    pub sources: Vec<OsString>,
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
        CONFIG.patch
    }
    pub(crate) fn convert() -> bool {
        CONFIG.convert
    }

    pub(crate) fn fail_fast() -> bool {
        CONFIG.fail_fast
    }
    pub(crate) fn update_patch_set() -> bool {
        CONFIG.update_patch_set
    }

    pub(crate) fn install() -> Option<&'static Path> {
        if CONFIG.install {
            Some(&CONFIG.install_location)
        } else {
            None
        }
    }

    pub(crate) fn generate_patches() -> Option<&'static PatchesGenerateOptions> {
        CONFIG.patches.as_ref().map(|arg| {
            let PatchesCommand::Patches(arg) = arg;
            let PatchesSubCommandAction::Generate(arg) = &arg.action;
            arg
        })
    }

    pub(crate) fn imports_location() -> PathBuf {
        Self::output_dir()
            .parent()
            .expect("config output dir has parent")
            .join("imports.nu")
    }
}

fn xdg_config_path(subpath: impl AsRef<Path>) -> PathBuf {
    if let Ok(dir) = env::var("XDG_CONFIG_HOME").map(PathBuf::from) {
        dir.join(subpath)
    } else {
        env::var("HOME")
            .map(PathBuf::from)
            .expect("$HOME environment variable to be set")
            .join(".config")
            .join(subpath)
    }
}
