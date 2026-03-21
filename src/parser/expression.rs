//! Parses expressions

use crate::{
    error::ErrorStack,
    parser::token::process_token,
    token::{RealToken, TokenList},
    parser::ast::Expression
};

pub fn consume_expression(input: &TokenList, i: usize) -> Result<(usize, Expression), ErrorStack> {
    // read all tokens to line end or "#" (after it's an inline comment).

    let list: Expression = input.1.iter().skip(i).map_while(|token| {
        if let RealToken::Unknown(ref k) = token.0 {
            (!k.starts_with("#"))
                .then(|| process_token(token))
        } else if matches!(token.0, RealToken::NewLine) {
            None
        } else {
            Some(process_token(token))
        }
    }).collect();

    Ok((list.len(), list))
}
