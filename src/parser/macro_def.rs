//! Parses macro definitions

use crate::{
    parser::{
        utils::{consume_identifier, consume_static},
        ast::MacroStatement,
        expression::consume_expression
    },
    error::ErrorStack,
    token::TokenList
};
use crate::error::ErrorKind;
use crate::parser::KEYWORDS;

pub fn consume_macro(input: &TokenList, i: usize) -> Option<Result<(usize, MacroStatement), ErrorStack>> {
    // <name> ":" expression
    
    let colon = consume_static(input, i + 1, ":")?;
    let (start_pos, name) = match consume_identifier(input, i) {
        Ok(kv) => kv,
        Err(e) => return Some(Err(e)),
    };
    
    if KEYWORDS.contains(&name.as_ref()) {
        let range = input.1[i].clone().into();
        
        return Some(Err(ErrorStack::new(
            ErrorKind::IllegalMacroName,
            input.0.clone(),
            range
        )));
    }
    
    if name.starts_with("$") {
        let range = (start_pos..start_pos + 1).into();
        
        return Some(Err(ErrorStack::new(
            ErrorKind::UnexpectedToken,
            input.0.clone(),
            range
        )));
    }
    
    let expression = consume_expression(input, i + 2);
    
    Some(match expression {
        Ok((size, expression)) => {
            let end_pos = expression.last().unwrap_or(&colon).position.1;
            
            Ok((size + 2, MacroStatement {
                name,
                range: (start_pos..end_pos).into(),
                expression,
            }))
        }
        Err(err) => Err(err),
    })
}
