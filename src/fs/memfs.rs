//! Virtual file system adapter for Fers runtime.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use crate::fs::FileSystem;

pub struct MemFS {
    files: HashMap<PathBuf, String>,
}

impl MemFS {
    pub fn new() -> Self {
        Self { files: HashMap::new() }
    }
}

impl FileSystem for MemFS {
    fn read_file(&self, path: &Path) -> io::Result<String> {
        self.files.get(path).cloned().ok_or_else(|| io::Error::new(
            io::ErrorKind::NotFound, 
            format!("File not found: {}", path.display())
        ))
    }
}
