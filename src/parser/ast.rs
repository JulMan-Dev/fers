//! The AST

use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use rust_decimal::Decimal;
use crate::position::PositionRange;

#[derive(Debug, Clone)]
pub struct Chunk {
    pub source: Rc<str>,
    pub statements: Vec<Statement>,
}

impl Chunk {
    pub fn position(&self) -> Option<(usize, usize)> {
        let first = self.statements.first()?;
        let last = self.statements.last()?;
            
        Some((first.position().unwrap().0, last.position().unwrap().1))
    }
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

impl Statement {
    pub fn position(&self) -> Option<(usize, usize)> {
        let range = match self {
            Statement::Comment(stmt) => stmt.range,
            Statement::Blank(s) => return Some((*s, s + 1)),
            Statement::Meta(stmt) => stmt.range,
            Statement::Macro(stmt) => stmt.range,
            Statement::Variable(stmt) => stmt.range,
            Statement::Expression(stmt) => stmt.range,
        };
        
        let start = range.start();
        let end = range.end()?;
        Some((start, end))
    }
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

#[derive(Debug, Clone)]
pub struct ClosureExpression {
    pub parameters: Rc<[Rc<str>]>,
    pub body: Rc<Chunk>,
}

#[derive(Debug, Clone)]
pub struct CallExpression {
    pub callee: Expression,
}

#[derive(Debug, Clone)]
pub struct Expression(pub Vec<TokenMeta>);

impl Expression {
    pub fn position(&self) -> Option<PositionRange> {
        let first = self.0.first()?;
        let last = self.0.last()?;
        
        Some((first.position.0..last.position.1).into())
    }
    
    pub fn new() -> Self {
        Self(Vec::new())
    }
}

impl Deref for Expression {
    type Target = Vec<TokenMeta>;
    
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Expression {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Vec<TokenMeta>> for Expression {
    fn from(tokens: Vec<TokenMeta>) -> Self {
        Self(tokens)
    }
}

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
    Closure(ClosureExpression),
    Call(CallExpression),
}
