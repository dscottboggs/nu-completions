use std::{
    path::Path,
    process::{Command, Stdio},
};

use anyhow::Result;
use log::{debug, info, trace, warn};

use crate::config::Config;

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
    // for def in Config::output_dir().read_dir()? {
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
            let _def = source.with_extension("nu");
            let def = _def
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
