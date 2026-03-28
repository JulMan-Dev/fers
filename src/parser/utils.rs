use std::rc::Rc;
use crate::error::{ErrorKind, ErrorStack};
use crate::parser::ast::{Token, TokenMeta};
use crate::parser::token::process_token;
use crate::token::{LexToken, RealToken, TokenList};

pub fn consume_static(input: &TokenList, i: usize, text: &str) -> Result<TokenMeta, ErrorStack> {
    let token = input.1.get(i);
    
    if let Some(token) = token {
        if let LexToken(RealToken::Unknown(k), _, _) = token && k.as_ref() == text {
            Ok(process_token(token))
        } else {
            Err(ErrorStack::new(
                ErrorKind::UnexpectedToken,
                input.0.clone(),
                token.clone().into(),
            ))
        }
    } else {
        Err(ErrorStack::new(
            ErrorKind::UnexpectedEndOfFile,
            input.0.clone(),
            (input.0.len() - 1..input.0.len()).into(),
        ))
    }
}

#[macro_export]
macro_rules! consume_static {
    ($input:expr, $i:expr, $text:expr) => {
        match consume_static($input, $i, $text) {
            Ok(node) => node,
            Err(err) => return Some(Err(err)),
        }
    };
}

pub fn consume_identifier(input: &TokenList, i: usize) -> Result<(usize, Rc<str>), ErrorStack> {
    if let Some(lex_token) = input.1.get(i) {
        let LexToken(token, _, _) = lex_token;

        if matches!(token, RealToken::NewLine) {
            return Err(ErrorStack::new(
                ErrorKind::UnexpectedEndOfLine,
                input.0.clone(),
                lex_token.into(),
            ));
        }

        let content = process_token(lex_token);

        if let Token::Identifier(name) = content.token {
            Ok((1, name))
        } else {
            Err(ErrorStack::new(
                ErrorKind::UnexpectedToken,
                input.0.clone(),
                lex_token.into(),
            ))
        }
    } else {
        Err(ErrorStack::new(
            ErrorKind::UnexpectedEndOfFile,
            input.0.clone(),
            (..input.0.len() - 1).into(),
        ))
    }
}
