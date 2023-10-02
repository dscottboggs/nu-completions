#![feature(lazy_cell, once_cell, never_type, exit_status_error, async_closure, let_chains)]
mod completion_line;
mod completions;
mod config;
mod dir_walker;
mod nu;
mod patching;
use beau_collector::BeauCollector as _;
use patching::fetch_latest_patch_set;

use std::{
    fs::{create_dir, File},
    io::{BufRead, BufReader, Seek, Write},
    path::{Path, PathBuf},
    sync::LazyLock,
};

use config::Config;
use log::{debug, info, trace};

use crate::nu::{processing_failed, CompletionsProcessor};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    femme::with_level(Config::verbose().log_level_filter());
    let mut conversion_errors: Vec<Result<_, _>> = vec![];

    if let Some(options) = Config::generate_patches() {
        patching::generate_patches(options)?;
    } else if let Some(install_location) = Config::install() {
        install_config(install_location)?;
    } else {
        if Config::update_patch_set() {
            fetch_latest_patch_set().await?;
        }
        if Config::convert() {
            if !Config::output_dir().exists() {
                trace!(
                    "output directory '{:?}' does not exist, creating",
                    Config::output_dir()
                );
                create_dir(Config::output_dir())?;
                debug!("created output directory {:?}", Config::output_dir());
            }

            let processor = CompletionsProcessor::default();
            info!("beginning translation phase");
            for source in Config::sources() {
                let path: PathBuf = source.into();
                if let Err(err) = processor.process_file_or_dir(path) {
                    let result = processing_failed(source, err).map(|_| unreachable!());
                    if Config::fail_fast() {
                        trace!("failing fast");
                        return result;
                    } else {
                        trace!("deferring failure");
                        conversion_errors.push(result);
                    }
                }
            }
            debug!(error_count = conversion_errors.len(); "finished processing all translations");
            processor.write_sourcing_file(&Config::imports_location())?;
            info!("finished translation phase");
        }
        if Config::patch() {
            info!("beginning patch phase");
            patching::patch_all()?;
            info!("finished patching");
        }
        conversion_errors.into_iter().bcollect::<Vec<()>>()?;
    }
    Ok(())
}

/// Add a line to nu's env config to source the defined completions.
fn install_config(location: &Path) -> anyhow::Result<()> {
    static CONFIG_DEF: LazyLock<String> =
        LazyLock::new(|| format!("source {:?}", Config::imports_location()));
    let mut file = File::options().read(true).write(true).open(location)?;
    file.seek(std::io::SeekFrom::Start(0))?;
    let buf_reader = BufReader::new(&file);
    for (n, line) in buf_reader.lines().enumerate() {
        let line = line?;
        if line.contains(&*CONFIG_DEF) {
            info!(line_number = n; "config found");
            return Ok(());
        }
    }
    file.seek(std::io::SeekFrom::End(0))?;
    file.write_all(b"\n")?;
    file.write_all(CONFIG_DEF.as_bytes())?;
    file.write_all(b"\n")?;
    debug!("config written");
    Ok(())
}
