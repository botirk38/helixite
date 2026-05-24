use heed::{Env, EnvOpenOptions};
use std::path::Path;

use crate::config::Config;
use crate::error::Result;

pub fn open_env(path: &Path, config: &Config) -> Result<Env> {
    std::fs::create_dir_all(path)?;

    let env = unsafe {
        EnvOpenOptions::new()
            .map_size(config.map_size)
            .max_dbs(config.max_dbs)
            .max_readers(config.max_readers)
            .open(path)?
    };

    Ok(env)
}
