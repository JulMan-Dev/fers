//! Real file system adapter for Fers runtime.

use std::fs::read_to_string;
use std::io;
use crate::fs::FileSystem;

pub struct RealFS;

impl RealFS {
    pub fn new() -> Self {
        Self {}
    }
}

impl FileSystem for RealFS {
    fn read_file(&self, path: &std::path::Path) -> io::Result<String> {
        read_to_string(path)
    }
}
