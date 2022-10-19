#![feature(once_cell, never_type, exit_status_error)]
mod completion_line;
mod completions;
mod config;
mod nu;
mod patching;

use std::{fs::create_dir, path::PathBuf};

use config::Config;
use log::{debug, info, trace};
use nu::{process_file, processing_failed};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // stderrlog::new()
    //     .quiet(Config::verbose().is_silent())
    //     .verbosity(Config::verbose().log_level().unwrap_or(log::Level::Error))
    //     .timestamp(stderrlog::Timestamp::Millisecond)
    //     .init()
    //     .expect("failed to initialize logger");
    femme::with_level(Config::verbose().log_level_filter());

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
        if let Err(err) = process_file(path).await {
            return processing_failed(source, err).map(|_| unreachable!());
        }
    }
    info!("finished translation phase, beginning patch phase");
    patching::patch_all()?;
    info!("finished patching");

    Ok(())
}
