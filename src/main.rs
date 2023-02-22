#![feature(once_cell, never_type, exit_status_error, async_closure, let_chains)]
mod completion_line;
mod completions;
mod config;
mod dir_walker;
mod nu;
mod patching;

use std::{fs::create_dir, path::PathBuf};

use config::Config;
use log::{debug, info, trace};
use nu::processing_failed;

use crate::nu::process_file_or_dir;

fn main() -> anyhow::Result<()> {
    femme::with_level(Config::verbose().log_level_filter());

    if let Some(options) = Config::generate_patches() {
        patching::generate_patches(options)?;
    } else {
        if Config::convert() {
            if !Config::output_dir().exists() {
                trace!(
                    "output directory '{:?}' does not exist, creating",
                    Config::output_dir()
                );
                create_dir(Config::output_dir())?;
                debug!("created output directory {:?}", Config::output_dir());
            }

            info!("beginning translation phase");
            for source in Config::sources().iter() {
                let path: PathBuf = source.into();
                if let Err(err) = process_file_or_dir(path) {
                    return processing_failed(source, err).map(|_| unreachable!());
                }
            }
            info!("finished translation phase");
        }
        if Config::patch() {
            info!("beginning patch phase");
            patching::patch_all()?;
            info!("finished patching");
        }
    }
    Ok(())
}
