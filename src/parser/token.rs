//! Parses tokens

use crate::{
    parser::{KEYWORDS},
    token::{LexToken, RealToken}
};
use crate::parser::ast::{Token, TokenMeta};

pub fn process_token(token: &LexToken) -> TokenMeta {
    let LexToken(token, start_pos, end_pos) = token;
    
    match token {
        RealToken::NewLine => unreachable!(),
        RealToken::Unknown(s) => {
            let token = if KEYWORDS.contains(&s.as_ref()) {
                Token::Identifier(s.clone())
            } else {
                let result = (s.as_ref() == "null")
                    .then_some(Token::Null)
                    .or_else(|| s.parse().map(Token::Boolean).ok())
                    .or_else(|| s.parse().map(Token::Integer).ok())
                    .or_else(|| s.parse().map(Token::Float).ok());

                result.unwrap_or_else(|| Token::Identifier(s.clone()))
            };

            TokenMeta {
                position: (*start_pos, *end_pos),
                token,
            }
        }
        RealToken::String(s) => TokenMeta {
            token: Token::String(s[1..s.len() - 1].into()),
            position: (*start_pos, *end_pos),
        },
    }
}
