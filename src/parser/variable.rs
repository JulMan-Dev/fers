//! Parses variable declarations

use crate::{
    parser::{
        utils::{consume_identifier, consume_static},
        expression::consume_expression,
        ast::VariableStatement
    },
    error::{ErrorKind, ErrorStack},
    token::TokenList
};

pub fn consume_variable(input: &TokenList, i: usize) -> Option<Result<(usize, VariableStatement), ErrorStack>> {
    let r#let = consume_static(input, i, "let")?;
    let start_pos = r#let.position.0;
    
    let (_, name) = match consume_identifier(input, i + 1) {
        Ok(kv) => kv,
        Err(e) => return Some(Err(e)),
    };
    
    if !name.starts_with('$') || name.as_ref() == "$" {
        return Some(Err(ErrorStack::new(
            ErrorKind::UnexpectedToken,
            input.0.clone(),
            input.1[i + 1].clone().into(), // safety "consume_identifier"
        )));
    }
    
    let equal = consume_static(input, i + 2, "=")?;
    
    let (size, expression) = match consume_expression(input, i + 3) {
        Ok(kv) => kv,
        Err(e) => return Some(Err(e)),
    };
    
    let end_pos = expression.last().unwrap_or(&equal).position.1;
    
    Some(Ok((size + 3, VariableStatement {
        name,
        expression,
        range: (start_pos..end_pos).into(),
    })))
}
