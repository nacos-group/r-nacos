use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct TempFile {
    pub(crate) path: PathBuf,
}

impl TempFile {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        if !self.path.exists() {
            return;
        }
        if self.path.is_dir() {
            fs::remove_dir_all(&self.path).ok();
        } else {
            fs::remove_file(&self.path).ok();
        }
    }
}
