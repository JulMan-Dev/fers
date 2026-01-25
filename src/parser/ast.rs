//! The AST

use std::rc::Rc;
use rust_decimal::Decimal;
use crate::position::PositionRange;

#[derive(Debug, Clone)]
pub struct Chunk {
    pub source: Rc<str>,
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub enum Statement {
    /// A comment line, does nothing.
    Comment(CommentStatement),
    /// A blank line, does nothing.
    Blank(usize),
    /// A meta statement, it didacts action to do for runtime, like including files.
    Meta(MetaStatement),
    /// A macro definition line, does nothing until expanded.
    Macro(MacroStatement),
    /// A variable definition, expression is executed and resultant stack is stored.
    Variable(VariableStatement),
    /// An expression statement, executed unconditionally.
    Expression(ExpressionStatement),
}

#[derive(Debug, Clone)]
pub struct CommentStatement {
    pub content: Rc<str>,
    pub range: PositionRange,
}

#[derive(Debug, Clone)]
pub struct MacroStatement {
    pub name: Rc<str>,
    pub range: PositionRange,
    pub expression: Expression,
}

#[derive(Debug, Clone)]
pub struct MetaStatement {
    pub name: Rc<str>,
    pub arguments: Expression,
    pub range: PositionRange,
}

#[derive(Debug, Clone)]
pub struct VariableStatement {
    pub name: Rc<str>,
    pub range: PositionRange,
    pub expression: Expression,
}

#[derive(Debug, Clone)]
pub struct ExpressionStatement {
    pub range: PositionRange,
    pub expression: Expression,
}

pub type Expression = Vec<TokenMeta>;

#[derive(Debug, Clone)]
pub struct TokenMeta {
    pub token: Token,
    pub position: (usize, usize),
}

#[derive(Debug, Clone)]
pub enum Token {
    Identifier(Rc<str>),
    Null,
    Integer(i128),
    Float(Decimal),
    String(Rc<str>),
    Boolean(bool),
}
