use anyhow::{anyhow, Result};
use async_recursion::async_recursion;
use std::{
    fmt::Debug,
    future::Future,
    path::{Path, PathBuf},
};

#[async_recursion]
pub async fn walk_dir<T, F, CbFut>(path: &Path, extra_arg: T, callback: F) -> Result<()>
where
    F: Fn(PathBuf, T) -> CbFut + Send + Sync,
    CbFut: Future<Output = Result<()>> + Send + Sync,
    T: Clone + Debug + Send + Sync,
{
    // let path = path.as_ref();
    if path.is_dir() {
        for dir in path.read_dir()? {
            walk_dir(dir?.path().as_path(), extra_arg.clone(), &callback).await?
        }
        Ok(())
    } else if path.is_symlink() {
        walk_dir(
            &path
                .read_link()
                .map_err(|e| anyhow!("error dereferencing symlink at {path:?}: {e:?}"))?,
            extra_arg.clone(),
            callback,
        )
        .await
    } else if path.is_file() {
        callback(path.to_path_buf(), extra_arg.clone()).await
    } else if path.exists() {
        Err(anyhow!("incompatible file: {path:?} exists, but is neither a directory, a symlink, nor a file."))
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("{path:?} does not exist"),
        )
        .into())
    }
}
