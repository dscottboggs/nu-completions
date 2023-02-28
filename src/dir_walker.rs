use anyhow::{anyhow, Result};
use std::{
    fmt::Debug,
    path::{Path, PathBuf},
};

pub fn walk_dir<T, F>(path: &Path, extra_arg: T, callback: F) -> Result<()>
where
    F: Fn(PathBuf, T) -> Result<()> + Send + Sync + Clone,
    T: Clone + Debug + Send + Sync,
{
    if path.is_dir() {
        for dir in path.read_dir()? {
            walk_dir(dir?.path().as_path(), extra_arg.clone(), callback.clone())?;
        }
        Ok(())
    } else if path.is_symlink() {
        walk_dir(
            &path
                .read_link()
                .map_err(|e| anyhow!("error dereferencing symlink at {path:?}: {e:?}"))?,
            extra_arg,
            callback,
        )
    } else if path.is_file() {
        callback(path.to_path_buf(), extra_arg)
    } else if path.exists() {
        // I will be surprised if we encounter either of these two final cases.
        Err(anyhow!("incompatible file: {path:?} exists, but is neither a directory, a symlink, nor a file."))
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("{path:?} does not exist"),
        )
        .into())
    }
}
