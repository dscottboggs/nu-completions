use std::{
    fs::File,
    io,
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{anyhow, Result};
use log::{as_debug, debug, error, info, trace, warn};
use tempfile::tempdir;

use crate::{
    config::{self, Config},
    dir_walker::walk_dir,
    nu::{process_file_given_output_dir, processing_failed},
};

pub(crate) fn patch(source: impl AsRef<Path>, patch: impl AsRef<Path>) -> Result<()> {
    let source = source.as_ref().as_os_str();
    let patch = patch.as_ref().as_os_str();
    let result = Command::new("patch")
        .arg(source)
        .arg(patch)
        .stdout(Stdio::piped())
        .output()?;

    let patch_output = String::from_utf8_lossy(&result.stdout);
    result
        .status
        .exit_ok()
        .map(|_| {
            debug!(
                source = source.to_string_lossy(),
                patch = patch.to_string_lossy(),
                patch_output = patch_output;
                "successfully patched"
            );
        })
        .map_err(|status| {
            warn!(
                source = source.to_string_lossy(),
                patch = patch.to_string_lossy(),
                patch_output = patch_output,
                status = result.status.code();
                "error patching"
            );
            status.into()
        })
}

pub(crate) fn patch_all() -> Result<()> {
    for source in Config::sources() {
        trace!(source = source.to_string_lossy(); "checking for patches");
        let source: &Path = source.as_ref();
        let _patch = source.with_extension("patch");
        let Some(patch_file) = _patch.file_name() else {
            debug!(source = source.to_string_lossy(); "failed to get file name with patch extension");
            continue;
        };
        let patch_file = Config::patch_dir().join(patch_file);
        if patch_file.exists() {
            debug!(
                patch_file = patch_file.to_string_lossy(),
                source = source.to_string_lossy();
                "found patch file"
            );
            let def = source.with_extension("nu");
            let def = def
                .file_name()
                .expect("to be able to extract file name from path"); // this is already checked for
            let def = Config::output_dir().join(def);
            if def.exists() {
                patch(def, patch_file)?;
            } else {
                info!(
                    source = source.to_string_lossy(),
                    patch_file = patch_file.to_string_lossy();
                    "source and patch found, but no converted definition. Perhaps conversion failed?"
                );
            }
        } else {
            trace!(source = source.to_string_lossy(), patch = patch_file.to_string_lossy(); "no patches found");
        }
    }
    Ok(())
}

fn generate_patch(source: &Path, generated: &Path, destination: &Path) -> Result<()> {
    let mut process = Command::new("diff") // generate a diff
        .arg(generated) // between the freshly generated translation
        .arg(source) // and the version you've modified...
        .stdout(Stdio::piped())
        .spawn()?;
    match File::create(destination) {
        Ok(mut destination) => {
            // ...then write the diff to a patch file
            io::copy(
                process
                    .stdout
                    .as_mut()
                    .expect("stdout requested but not given"),
                &mut destination,
            )?;
            process.wait()?.exit_ok()?;
            Ok(())
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
}

pub(crate) fn generate_patches(opts: &config::PatchesGenerateOptions) -> Result<()> {
    let freshly_generated_store = tempdir()?;
    for source in Config::sources().iter() {
        walk_dir(
            source.as_ref(),
            freshly_generated_store.path(),
            |path, freshly_generated_store| {
                let result = process_file_given_output_dir(&path, freshly_generated_store);
                let freshly_generated = match result {
                    Ok(freshly_generated) => freshly_generated,
                    Err(err) => {
                        return processing_failed(source, err).map(|_| unreachable!());
                    }
                };

                generate_patch(
                    &path,
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
    Ok(())
}
