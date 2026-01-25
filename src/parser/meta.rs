//! Parses meta statements

use crate::{
    error::{ErrorKind, ErrorStack},
    parser::{ast::{MetaStatement, Token}, token::process_token},
    token::{TokenList, LexToken, RealToken},
    parser::expression::consume_expression
};

pub fn consume_meta_statement(input: &TokenList, i: usize) -> Option<Result<(usize, MetaStatement), ErrorStack>> {
    // ":" <name> expression
    
    let Some(token) = input.1.get(i) else {
        return None;
    };
    
    if let LexToken(RealToken::Unknown(k), start_pos, _) = token && k.as_ref() == ":" {
        let next = input.1.get(i + 1);
        
        if matches!(next, None | Some(LexToken(RealToken::NewLine, _, _))) {
            return Some(Err(ErrorStack::new(
                ErrorKind::UnexpectedEndOfLine,
                input.0.clone(),
                (token.1 + 1..).into(),
            )));
        }
        
        let next = next.unwrap();
        let content = process_token(next);
        
        if let Token::Identifier(ref name) = content.token {
            let expr = consume_expression(input, i + 2);
            
            Some(match expr {
                Ok((size, arguments)) => {
                    let end_pos = arguments.last().unwrap_or(&content).position.1;
                    
                    Ok((size + 2, MetaStatement {
                        name: name.clone(),
                        arguments,
                        range: (*start_pos..end_pos).into(),
                    }))
                }
                Err(err) => Err(err)
            })
        } else {
            Some(Err(ErrorStack::new(
                ErrorKind::UnexpectedToken,
                input.0.clone(),
                next.into(),
            )))
        }
    } else {
        None
    }
}
