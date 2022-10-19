use std::{
    path::Path,
    process::{Command, Stdio},
};

use anyhow::Result;
use log::{debug, warn};

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
    if let Err(status) = result.status.exit_ok() {
        warn!(
            source = source.to_string_lossy(),
            patch = patch.to_string_lossy(),
            patch_output = patch_output,
            status = result.status.code();
            "error patching"
        );
        return Err(status.into());
    } else {
        debug!(
            source = source.to_string_lossy(),
            patch = patch.to_string_lossy(),
            patch_output = patch_output;
            "successfully patched"
        );
    }

    Ok(())
}

pub(crate) fn patch_all() -> Result<()> {
    for def in Config::output_dir().read_dir()? {
        let def = def?;
        if let Some(patchfile) = def.path().with_extension("patch").file_name() {
            let patch_file = Config::patch_dir().join(patchfile);
            if patch_file.exists() {
                patch(def.path(), patch_file)?;
            }
        }
    }
    Ok(())
}
