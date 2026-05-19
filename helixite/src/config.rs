#[derive(Debug, Clone)]
pub struct Config {
    pub map_size: usize,
    pub max_dbs: u32,
    pub max_readers: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            map_size: 1024 * 1024 * 1024,
            max_dbs: 32,
            max_readers: 126,
        }
    }
}

impl Config {
    pub fn with_map_size(mut self, bytes: usize) -> Self {
        self.map_size = bytes;
        self
    }
}
