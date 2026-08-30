use std::collections::VecDeque;
use std::rc::Rc;
use crate::consume_static;
use crate::error::{ErrorKind, ErrorStack};
use crate::parser::ast::{Expression, TokenMeta};
use crate::parser::expression::consume_expression;
use crate::parser::utils::{consume_identifier, consume_static};
use crate::position::PositionRange;
use crate::token::{RealToken, TokenList};

pub fn consume_string_template(input: &TokenList, i: usize) -> Option<Result<usize, ErrorStack>> {
    /*
    A string template can be thought of a string with interpolations, just like JavaScript 
    string templating: `hello ${name}` or Python f-string: f"hello {name}".
    
    But the syntax in Fers is a bit different, we use:
    
      let $message = `Hello, \($name)!`
      
    We use backtick (`) to denote the string template and use `\(` to denote interpolation.
    Some can think it may be syntactical sugar, but it's not, it's not equivalent to:
    
      let $message = "Hello, " $name string + "!" +
      
    For templating, the runtime replaces the interpolation with the VM's representation of 
    the value.
    
    For example:
    
      let $closure = () -> {}
      let $msg1 = `Hello, \($closure)!`
      let $msg2 = "Hello, " $closure string + "!" +
    
    Both look the same, but if we inspect the values:
    
      $msg1: String("Hello, closure!")
      $msg2: String("Hello, null!")
      
    Pretty cool, right? Being smaller and better.
    */
    
    let first = consume_static(input, i, "`").ok()?;
    let first_pos = first.position.0;
    
    let mut segments = VecDeque::new();
    
    let mut iter = input.1.iter().enumerate().skip(i + 2);
    
    let done = loop {
        if let Some((j, token)) = iter.next() {
            if token.0 == RealToken::NewLine {
                // unexpected new line, unclosed string literal error
                return Some(Err(ErrorStack::new(
                    ErrorKind::UnclosedStringLiteral,
                    input.0.clone(),
                    (first_pos..token.2).into(),
                )))
            }

            if consume_static(input, j, "`").is_ok() {
                break Some(j);
            };

            let Ok(escape) = consume_static(input, j, "\\") else {
                let start_pos: usize = segments.back()
                    .map(|s: &Segment| s.start())
                    .unwrap_or(first_pos);
                segments.push_back(Segment::Text(start_pos, token.2));
                continue;
            };

            // we need to check if the next token is:
            // - "(", indicating an interpolation
            // - "\\", indicating a backslash
            // - "`", indicating a backtick
            // - "n", indicating a newline
            // - "t", indicating a tab
            // - else, throwing UnknownEscapeSequence
            
            let s: Result<(), PositionRange> = 'process_escape: {
                let Some((j, next_token)) = iter.next() else {
                    break 'process_escape Err((input.0.len() - 1..).into());
                };

                if token.2 != next_token.2 {
                    // the next token is not the very next, invalid escape sequence,
                    break 'process_escape Err((token.2..next_token.2).into());
                }

                let RealToken::Unknown(ref s) = next_token.0 else {
                    break 'process_escape Err((token.2..next_token.2).into());
                };
                
                if s.as_ref() == "(" {
                    // starting consuming an expression, yay!
                    
                    let (consumed, expression) = match consume_expression(input, j) {
                        Ok(expr) => expr,
                        Err(err) => return Some(Err(err)),
                    };
                    
                    if consumed == 0 || expression.len() == 0 {
                        // no tokens in the expression mean empty interpolation,
                        // or the expression is invalid, throw error
                        break 'process_escape Err((token.2..token.2).into());
                    }
                    
                    let (j, token_after_expr) = iter.nth(consumed - 1).unwrap();
                    // nth skips consumed - 1 and call next, so we are on j + consumed token,
                    // normally we are currently on ")" if the interpolation is valid.
                    // SAFETY: we can use unwrap here because we know the expression is valid
                    //         and tokens exist.
                    
                    if token_after_expr.0 == RealToken::NewLine {
                        return Some(Err(ErrorStack::new(
                            ErrorKind::UnexpectedEndOfLine,
                            input.0.clone(),
                            (token_after_expr.1..token_after_expr.2).into(),
                        )));
                    }
                    
                    // Notice that we know that the token on j is ")" because 
                    // consume_expression early returns Ok when it encounters ")"
                    // NewLine was checked just before.
                    
                    let last_pos = token_after_expr.2;
                    
                    segments.push_back(Segment::Interpolation(expression, token.1, last_pos));
                    break 'process_escape Ok(());
                }
              
                // checking "n", "t", "\\", "`""
                let Some((j, next_token)) = iter.next() else {
                    return Some(Err(ErrorStack::new(
                        ErrorKind::UnclosedStringLiteral,
                        input.0.clone(),
                        (first_pos..).into(),
                    )));
                };
                
                todo!();
            };
            
            if let Err(pos) = s {
                return Some(Err(ErrorStack::new(
                    ErrorKind::UnknownEscapeSequence, input.0.clone(), pos)));
            };
        } else {
            break None;
        }
    };
    
    todo!();
}

#[derive(Debug)]
enum Segment {
    Text(usize, usize),
    Literal(Rc<str>, usize, usize),
    Interpolation(Expression, usize, usize),
}

impl Segment {
    pub fn start(&self) -> usize {
        match self {
            Segment::Text(start, _) => *start,
            Segment::Literal(_, start, _) => *start,
            Segment::Interpolation(_, start, _) => *start,
        }
    }

    pub fn end(&self) -> usize {
        match self {
            Segment::Text(_, end) => *end,
            Segment::Literal(_, _, end) => *end,
            Segment::Interpolation(_, _, end) => *end,
        }
    }
}
