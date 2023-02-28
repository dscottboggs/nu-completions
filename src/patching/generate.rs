use std::{
    fs::File,
    io::{self, Read, Write},
    path::Path,
    process::{Command, Stdio},
    sync::{Arc, RwLock},
};

use crate::{
    config::{self, Config},
    dir_walker::walk_dir,
    nu::{processing_failed, CompletionsProcessor},
};

use anyhow::{anyhow, Result};
use beau_collector::BeauCollector as _;
use log::{as_debug, debug, error, trace};
use tempfile::tempdir;

/// Write the difference between `source` and `generated` to `destination`.
fn generate_patch(source: &Path, generated: &Path, destination: &Path) -> Result<()> {
    trace!(
        source = as_debug!(source), generated = as_debug!(generated),
        destination = as_debug!(destination);
        "checking for file differences"
    );
    let mut process = Command::new("diff") // generate a diff
        .arg(generated) // between the freshly generated translation
        .arg(source) // and the version you've modified...
        .stdout(Stdio::piped())
        .spawn()?;
    let mut stdout = process.stdout.take().unwrap();
    let mut buf = [0u8; 0x100];
    let first_read = {
        let n = stdout.read(&mut buf)?;
        &buf[..n]
    };
    if !first_read.is_empty() {
        trace!(first_read = first_read.len(); "read bytes from stdout, writing diff to file");
        match File::create(destination) {
            Ok(mut destination) => {
                // ...then write the diff to a patch file
                destination.write_all(first_read)?;
                io::copy(&mut stdout, &mut destination)?;
                let status = process.wait()?;
                if let Some(code) = status.code() && code < 2 {
                    // Exit code 1 indicates success, files differ. 0 indicates
                    // success, no difference. Greater than that is error.
                    Ok(())
                } else {
                    Ok(status.exit_ok()?)
                }
            }
            Err(e) => {
                error!(error = as_debug!(e); "error generating patch");
                Err(if let Err(err_killing) = process.kill() {
                    error!(error = as_debug!(err_killing); "error stopping diff process");
                    anyhow!("while processing this error:\n\t{e}\nanother error occurred:\n\t{err_killing:?}")
                } else {
                    e.into()
                })
            }
        }
    } else {
        debug!(source = as_debug!(source); "source file did not differ");
        Ok(())
    }
}

/// Output the differences between the definitions defined at `opts.from` and
/// the definitions generated from `opts.sources` into the `opts.to` directory.
pub(crate) fn generate_patches(opts: &config::PatchesGenerateOptions) -> Result<()> {
    let freshly_generated_store = tempdir()?;
    trace!(
        sources = opts.sources.len(),
        from = as_debug!(opts.from),
        to = as_debug!(opts.to),
        "temp dir" = as_debug!(freshly_generated_store.path());
        "generating patches"
    );
    let processor = CompletionsProcessor::default();
    let regeneration_errors: Arc<RwLock<Vec<Result<()>>>> = Default::default();
    for source in opts.sources.iter() {
        walk_dir(
            source.as_ref(),
            freshly_generated_store.path(),
            |path, freshly_generated_store| {
                trace!(path = as_debug!(path); "generating patch");
                let result =
                    processor.process_file_given_output_dir(&path, freshly_generated_store);
                let freshly_generated = match result {
                    Ok(freshly_generated) => freshly_generated,
                    Err(err) => {
                        if Config::fail_fast() {
                            return processing_failed(source, err).map(|_| unreachable!());
                        } else {
                            regeneration_errors
                                .write()
                                .expect("rwlock write access")
                                .push(Err(err));
                            return Ok(());
                        }
                    }
                };
                let Some(file_name) = path.file_name() else {
                    error!(path = as_debug!(path); "expected file to have filename");
                    return Err(anyhow!("expected {path:?} to have filename"));
                };
                let modified_source = opts.from.join(file_name).with_extension("nu");

                generate_patch(
                    &modified_source,
                    &freshly_generated,
                    &opts.to.join(
                        freshly_generated
                            .with_extension("patch")
                            .file_name()
                            .ok_or_else(|| anyhow!("file had no name: {freshly_generated:?}"))?,
                    ),
                )
            },
        )?;
    }
    // I can't believe this fucking works. I mean, that it's necessary is it's
    // own absurdity, but that there's no abstraction over this and the whole
    // song-and-dance actually solves the whole "can't move from arc" thing? wtf
    let mut regeneration_errors_2 = vec![];
    regeneration_errors
        .write()
        .expect("rwlock write access")
        .drain(..)
        .for_each(|result| regeneration_errors_2.push(result));
    regeneration_errors_2.into_iter().bcollect::<Vec<()>>()?;
    Ok(())
}
