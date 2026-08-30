//! Parses expressions

use crate::{
    error::{ErrorStack, ErrorKind},
    parser::token::process_token,
    token::{RealToken, TokenList},
    parser::ast::Expression,
    parser::closure::consume_closure_or_call,
};
use crate::parser::string_templating::consume_string_template;

pub fn consume_expression(input: &TokenList, i: usize) -> Result<(usize, Expression), ErrorStack> {
    // read all tokens to line end, "#" (after it's an inline comment) or ")".
    
    // check if the first token is a ")", error if it is.
    if let Some(token) = input.1.get(i) && let RealToken::Unknown(ref k) = token.0 {
        if k.as_ref() == ")" {
            return Err(ErrorStack::new(
                ErrorKind::UnexpectedToken,
                input.0.clone(),
                token.clone().into(),
            ));
        }
        
        if k.as_ref() == "{" {
            return Err(ErrorStack::new(
                ErrorKind::EndOfBlock,
                input.0.clone(),
                token.clone().into(),
            ))
        }
    }

    let mut list = Expression::new();
    
    let mut consumed = 0;
    let mut i = i;
    while let Some(token) = input.1.get(i) {
        if let RealToken::Unknown(ref k) = token.0 {
            if k.as_ref() == "(" {
                if let Some(closure) = consume_closure_or_call(input, i) {
                    let (k, token) = closure?;
                    
                    list.push(token);
                    consumed += k;
                    i += k;
                    continue;
                }
            }
            
            if k.as_ref() == "`" {
                if let Some(closure) = consume_string_template(input, i) {
                    todo!();
                    continue;
                }
            }
            
            // parenthesis end (closure decl or call)
            if k.as_ref() == ")" {
                break;
            }
            
            // end of block
            if k.as_ref() == "}" {
                break;
            }
            
            if !k.starts_with("#") {
                list.push(process_token(token));
                consumed += 1;
                i += 1;
            } else {
                break;
            }
        } else if matches!(token.0, RealToken::NewLine) {
            break;
        } else {
            list.push(process_token(token));
            consumed += 1;
            i += 1;
        }
    };

    Ok((consumed, list))
}
