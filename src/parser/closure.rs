use std::rc::Rc;

use crate::{parser::{
    expression::consume_expression,
    consume_chunk,
    ast::{CallExpression, Expression, Token, TokenMeta, ClosureExpression},
    utils::consume_static
}, error::{ErrorKind, ErrorStack}, token::TokenList, consume_static};

impl ClosureExpression {
    /// Helper function to get arity of the closure.
    pub fn arity(&self) -> usize {
        self.parameters.len()
    }
}


// closure := "(" arguments * ")" "->" { statement * }
// call := "(" expr_node ")"
pub fn consume_closure_or_call(input: &TokenList, i: usize) -> Option<Result<(usize, TokenMeta), ErrorStack>> {
    let lparent = consume_static!(input, i, "(");
    let start_pos = lparent.position.0;
    
    let (parameters, consumed) = 'can_be_call: {
        // we check if i + 1 is ")"
        if consume_static(input, i + 1, ")").is_ok() {
            // we need to check if there is "->" after the ")"
            if consume_static(input, i + 2, "->").is_ok() {
                break 'can_be_call (Expression::new(), 2usize);
            }
         
            // cannot return an empty Call node, throw
            let last_token = input.1.get(i).unwrap();
            return Some(Err(ErrorStack::new(
                ErrorKind::UnexpectedToken,
                input.0.clone(),
                (start_pos..last_token.end_position()).into(),
            )));
        }

        // we need to consume an expression and check there is no "->" after

        let (consumed, expr) = match consume_expression(input, i + 1) {
            Ok(node) => node,
            // bubble up the error
            Err(err) => return Some(Err(err)),
        };
        
        consume_static!(input, i + consumed + 1, ")");
        
        // check if the after ")", there is "->"
        if consume_static(input, i + consumed + 2, "->").is_ok() {
            break 'can_be_call (expr, consumed + 2);
        }
        
        // we are sure that we are not in a closure declaration,
        // so we can return a "Call" node.
        
        let expr_position = expr.position().unwrap();
        let start_pos = start_pos;
        let end_pos = expr_position.end().unwrap_or(input.0.len() - 1) + 2;
        
        // "(" + expr + ")" = consumed + 2 tokens used. 
        return Some(Ok((
            consumed + 2,
            TokenMeta {
                token: Token::Call(CallExpression {
                    callee: expr,
                }),
                position: (start_pos, end_pos),
            }
        )));
    };
    
    let j = i + 1 + consumed;
    
    // we need to check if arguments are identifiers starting with "$".
    let parameters = if parameters.is_empty() {
        Vec::new()
    } else {
        let mut p = Vec::new();
        
        for (k, token) in parameters.iter().enumerate() {
            let index = j + k;
            
            if let Token::Identifier(var) = &token.token {
                if var.starts_with('$') && var.as_ref() != "$" {
                    p.push(var.to_owned());
                } else {
                    // raise an unexpected token
                    return Some(Err(ErrorStack::new(
                        ErrorKind::UnexpectedToken,
                        input.0.clone(),
                        (token.position.0..token.position.1).into(),
                    )));
                }
            } else {
                // required, enforcing an identifier.
                return Some(Err(ErrorStack::new(
                    ErrorKind::UnexpectedToken,
                    input.0.clone(),
                    (token.position.0..token.position.1).into(),
                )))
            }
        }

        p
    };
    
    // we need to consume a chunk (closure body)
    consume_static!(input, j, "{");
    let (body_consumed, body) = match consume_chunk(input, j + 1) {
        Ok(c) => c,
        Err(err) => return Some(Err(err)),
    };
    let close = consume_static!(input, j + 1 + body_consumed, "}");
    
    // fully parsed closure, return a new node.
    
    let parameters = Rc::from(parameters);
    let position = (start_pos, close.position.1 + 1);
    
    Some(Ok((
        body_consumed + 2 + j - i, // not index, but the consumed tokens.
        TokenMeta {
            token: Token::Closure(ClosureExpression {
                parameters,
                body: Rc::new(body),
            }),
            position,
        }
    )))
}
