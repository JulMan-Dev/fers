use std::path::Path;
use std::rc::Rc;
use crate::parser::ast::Chunk;

#[derive(Clone, Debug)]
pub struct StateFile {
    pub filename: Rc<Path>,
    pub source: Rc<str>,
    pub chunk: Option<Chunk>
}