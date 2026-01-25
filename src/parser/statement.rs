//! Parses statements

use std::rc::Rc;
use crate::error::{ErrorKind, ErrorStack};
use crate::parser::ast::{CommentStatement, ExpressionStatement, Statement};
use crate::parser::expression::consume_expression;
use crate::parser::macro_def::consume_macro;
use crate::parser::meta::consume_meta_statement;
use crate::parser::variable::consume_variable;
use crate::token::{LexToken, RealToken, TokenList};

pub fn consume_statement(input: &TokenList, i: usize) -> Result<(usize, Statement), ErrorStack> {
    // a statement can be: blank, comment, meta statement, macro definition, variable definition, expression
    // they are checked in order.
    
    // no more tokens to read
    if input.1.get(i).is_none() {
        return Err(ErrorStack::new(
            ErrorKind::EndOfFile,
            input.0.clone(),
            (0..).into(),
        ))
    }

    // Found blank line
    if consume_blank(input, i) {
        Ok((1, Statement::Blank(i)))
    } else if let Some((consumed, statement)) = consume_comment(input, i) {
        // Found comment
        Ok((consumed, Statement::Comment(statement)))
    } else if let Some(result) = consume_meta_statement(input, i) {
        // Found meta statement
        let (consumed, statement) = result?;
        
        Ok((consumed, Statement::Meta(statement))) 
    } else if let Some(result) = consume_macro(input, i) {
        // Found macro declaration 
        let (consumed, statement) = result?;
        
        Ok((consumed, Statement::Macro(statement)))
    } else if let Some(result) = consume_variable(input, i) {
        let (consumed, statement) = result?;
            
        Ok((consumed, Statement::Variable(statement))) 
    } else {
        // Found expression
        let (consumed, expression) = consume_expression(input, i)?;

        // note: first and last must exist until the line would be blank, this is case
        // is already checked before
        let start_pos = expression.first().unwrap().position.0;
        let end_pos = expression.last().unwrap().position.1;
        
        Ok((consumed, Statement::Expression(ExpressionStatement {
            expression,
            range: (start_pos..end_pos).into(),
        })))
    }
}

pub fn consume_blank(input: &TokenList, i: usize) -> bool {
    // "\n"
    
    let found = input.1.get(i);
    
    matches!(found, Some(LexToken(RealToken::NewLine, _, _)))
}

pub fn consume_comment(input: &TokenList, i: usize) -> Option<(usize, CommentStatement)> {
    // "#" <content> "\n"
    
    let found = input.1.get(i);
    
    if let Some(LexToken(RealToken::Unknown(k), start_pos, _)) = found && k.starts_with("#") {
        let vec = input.1
            .iter()
            .skip(i)
            .take_while(|v| !matches!(v.0, RealToken::NewLine))
            .collect::<Vec<_>>();
        let end_pos = vec.last().unwrap().2;
        let range = *start_pos..=end_pos;
        let content = Rc::from(&input.0[range.clone()]);
        
        Some((vec.len(), CommentStatement {
            content,
            range: range.into(),
        }))
    } else {
        None
    }
}
