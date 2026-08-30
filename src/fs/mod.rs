//! The file system adapter for Fers runtime.

pub mod memfs;
pub mod real;

use std::io;
use std::path::Path;

pub trait FileSystem {
    fn read_file(&self, path: &Path) -> io::Result<String>;
}
