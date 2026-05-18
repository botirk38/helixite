use heed::Env;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::error::Result;
use crate::storage::env::open_env;

pub struct Helixite {
    pub(crate) env: Env,
    pub(crate) path: PathBuf,
}

impl Helixite {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_with_config(path, Config::default())
    }

    pub fn open_with_config(path: impl AsRef<Path>, config: Config) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let env = open_env(&path, &config)?;
        Ok(Self { env, path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}
