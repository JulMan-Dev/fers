//! Raw tokens and tokens context manager.

use std::{fmt::Debug, iter::Peekable, rc::Rc, str::FromStr};

use crate::error::{ErrorKind, ErrorStack};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LexToken(pub RealToken, pub usize, pub usize);

impl LexToken {
    pub fn raw(&self) -> &str {
        match &self.0 {
            RealToken::NewLine => "\n",
            RealToken::Unknown(s) | RealToken::String(s) => &s,
        }
    }

    pub fn position(&self) -> usize {
        self.1
    }

    pub fn end_position(&self) -> usize {
        self.2
    }
    
    pub fn len(&self) -> usize {
        match &self.0 {
            RealToken::NewLine => 1,
            RealToken::Unknown(s) | RealToken::String(s) => s.len(),
        }
    }
}

impl Debug for LexToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("\"{}\" @ {}:{}", self.0.escape_default(), self.1, self.2))
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RealToken {
    NewLine,
    Unknown(Rc<str>),
    String(Rc<str>),
}

impl RealToken {
    pub fn escape_default(&self) -> String {
        match self {
            RealToken::NewLine => "\\n".to_string(),
            RealToken::Unknown(s) | RealToken::String(s) => s.escape_default().collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TokenList(pub Rc<str>, pub Vec<LexToken>);

fn is_char_word_break(c: char) -> bool {
    matches!(c, ':' | '=' | '(' | ')' | '&')
}

impl FromStr for TokenList {
    type Err = ErrorStack;

    #[doc = r"Try parse the string and returns the raw tokens tree from it."]
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut tokens = Vec::new();

        let mut iter: Peekable<_> = input.char_indices().peekable();

        'consumer: while let Some((position, char)) = iter.next() {
            if char == '\n' {
                tokens.push(LexToken(RealToken::NewLine, position, position + 1));
            }

            if char.is_whitespace() {
                continue;
            }
            
            // word breaker should be considered single token (excepted quote)
            if is_char_word_break(char) && char != '"' {
                tokens.push(LexToken(RealToken::Unknown(char.to_string().into()), position, position + 1));
                continue 'consumer;
            }

            let mut raw = String::from(char);

            match char {
                '#' => { // if matches!(tokens.last(), Some(LexToken(_, RealToken::NewLine))) => {
                    // line comment, consuming until new line
                    // analogous to last pattern but here we wait for '\n', not whitespace.
                    let mut last_pos = None;
                    while let Some(&(cur, char)) = iter.peek() {
                        if char == '\n' {
                            break;
                        }
                
                        raw.push(char);
                        last_pos.replace(cur);
                        let _ = iter.next();
                    }

                    let end_pos = last_pos.unwrap_or_else(|| {
                        position + raw.len()
                    });
                
                    tokens.push(LexToken(RealToken::Unknown(raw.into()), position, end_pos));
                }
                '"' => {
                    // consume iter until reaching end or closing quote.
                    while let Some((cur, char)) = iter.next() {
                        match char {
                            // end of string
                            '"' => {
                                raw.push(char);
                                tokens.push(LexToken(RealToken::String(raw.into()), position, cur));
                                continue 'consumer;
                            }
                            // escape
                            '\\' => match iter.next() {
                                None => {
                                    continue 'consumer;
                                }
                                Some((_, '"')) => raw.push('"'),
                                Some((_, '\\')) => raw.push('\\'),
                                Some((_, 'n')) => raw.push('\n'),
                                Some((_, 't')) => raw.push('\t'),
                                Some((next, _)) => {
                                    return Err(ErrorStack::new(
                                        ErrorKind::UnknownEscapeSequence,
                                        input.into(),
                                        (cur..=next).into(),
                                    ));
                                }
                            },
                            // new line, literal must be one-line
                            '\n' => break,
                            // any character
                            c => raw.push(c),
                        }
                    }

                    let mut latest_char = None::<(usize, char)>;

                    input
                        .char_indices()
                        .skip(position)
                        .inspect(|v| {
                            latest_char.replace(*v);
                        })
                        .skip_while(|x| x.1 != '\n')
                        .next();

                    let range = if let Some((pos, _)) = latest_char {
                        (position..pos).into()
                    } else {
                        (position..).into()
                    };

                    return Err(ErrorStack::new(
                        ErrorKind::UnclosedStringLiteral,
                        input.into(),
                        range,
                    ));
                }
                _ => {
                    let mut last_pos = None;
                    while let Some(&(cur, char)) = iter.peek() {
                        if char.is_whitespace() || is_char_word_break(char) {
                            break;
                        }
                        
                        raw.push(char);
                        last_pos.replace(cur);
                        let _ = iter.next();
                    }
                    
                    let end_pos = last_pos.unwrap_or_else(|| {
                        position + raw.len()
                    });

                    let raw = Rc::from(raw);
                    tokens.push(LexToken(RealToken::Unknown(raw), position, end_pos));
                }
            }
        }

        Ok(TokenList(input.into(), tokens))
    }
}
 
