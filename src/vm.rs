//! The VM for the tokens.

use std::{
    fmt::Display,
    io::{stdout, Write},
    rc::Rc,
};

use crate::{
    error::{ErrorKind, ErrorStack},
    token::TokenList,
    types::{Type, Value},
    utils::VecUtils,
};

#[doc = "A enum representing all operation that can be done inside of the default VM."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Operation {
    #[doc = r"Clone the last element."]
    Clone,
    #[doc = r"Get the type of the last element."]
    Type,
    #[doc = r"Add 2 elements together."]
    Add,
    #[doc = r"Substract 2 elements together."]
    Sub,
    #[doc = r"Multiply 2 elements together."]
    Mul,
    #[doc = r"Divide 2 elements together."]
    Div,
    #[doc = r"Try casting element to a given type."]
    CastTo(Type),
    #[doc = r"Evaluate the string into a sub VM."]
    Eval,
    #[doc = r"Parse the all string as one token."]
    Parse,
    #[doc = r"Write into the stdout the last token."]
    Write,
    #[doc = r"Stop the VM with an error."]
    Error,
    #[doc = r"Force stop the VM engine with an error message."]
    Panic,
}

impl Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&match self {
            Operation::Clone => "clone".into(),
            Operation::Type => "get type".into(),
            Operation::Add => "add".into(),
            Operation::Sub => "substract".into(),
            Operation::Mul => "multiply".into(),
            Operation::Div => "divide".into(),
            Operation::CastTo(t) => format!("cast to {t}"),
            Operation::Eval => "eval".into(),
            Operation::Parse => "parse".into(),
            Operation::Write => "write".into(),
            Operation::Error => "error".into(),
            Operation::Panic => "panic".into(),
        })
    }
}

/// Indicate a struct be executable.
pub trait Executable {
    #[doc = "Try executing this representation using a VM."]
    fn run(&self) -> Result<Vec<Value>, ErrorStack>;
}

impl Executable for TokenList {
    #[doc = "Try interpreting raw tokens inside of the default VM."]
    fn run(&self) -> Result<Vec<Value>, ErrorStack> {
        let mut stack: Vec<Value> = Vec::new();

        /* return Err(ErrorStack::with_cause(
            ErrorKind::UnclosedStringLiteral,
            ErrorStack::new(ErrorKind::InternalError, self.0.clone(), (&self.1[2]).into()),
            self.0.clone(),
            (0..10).into(),
        )); */

        for token in self.1.iter() {
            let enforce_arguments_count = |count: usize| -> Result<(), ErrorStack> {
                if stack.len() < count {
                    Err(ErrorStack::new(
                        ErrorKind::InvalidNumberArguments(count, stack.len()),
                        self.0.clone(),
                        token.into(),
                    ))
                } else {
                    Ok(())
                }
            };
            
            match token.raw() {
                "clone" => {
                    enforce_arguments_count(1)?;

                    stack.push(stack.get_last().clone());
                }
                "type" => {
                    enforce_arguments_count(1)?;

                    stack.push(stack.get_last().kind().to_string().into());

                    stack.remove_reversed(2);
                }
                "+" => {
                    enforce_arguments_count(2)?;

                    let right = {
                        let last = stack.get_last().clone();

                        stack.remove_last();

                        last
                    };
                    let left = {
                        let last = stack.get_last().clone();

                        stack.remove_last();

                        last
                    };

                    stack.push(match left + right {
                        Ok(res) => res,
                        Err(err) => return Err(ErrorStack::new(err, self.0.clone(), token.into())),
                    });
                }
                "-" => {
                    enforce_arguments_count(2)?;

                    let right = {
                        let last = stack.get_last().clone();

                        stack.remove_last();

                        last
                    };
                    let left = {
                        let last = stack.get_last().clone();

                        stack.remove_last();

                        last
                    };

                    stack.push(match left - right {
                        Ok(res) => res,
                        Err(err) => return Err(ErrorStack::new(err, self.0.clone(), token.into())),
                    });
                }
                "*" => {
                    enforce_arguments_count(2)?;

                    let right = {
                        let last = stack.get_last().clone();

                        stack.remove_last();

                        last
                    };
                    let left = {
                        let last = stack.get_last().clone();

                        stack.remove_last();

                        last
                    };

                    stack.push(match left * right {
                        Ok(res) => res,
                        Err(err) => return Err(ErrorStack::new(err, self.0.clone(), token.into())),
                    });
                }
                "/" => {
                    let Some([left, right]) = stack.take_last_chunk::<2>() else {
                        return Err(ErrorStack::new(
                            ErrorKind::InvalidNumberArguments(2, stack.len()),
                            self.0.clone(),
                            token.into(),
                        ));
                    };

                    stack.push(match left / right {
                        Ok(res) => res,
                        Err(err) => return Err(ErrorStack::new(err, self.0.clone(), token.into())),
                    });
                }
                "neg" => {
                    enforce_arguments_count(1)?;

                    let e = {
                        let last = stack.get_last().clone();

                        stack.remove_last();

                        last
                    };

                    stack.push(match -e {
                        Ok(res) => res,
                        Err(err) => return Err(ErrorStack::new(err, self.0.clone(), token.into())),
                    });
                }
                "integer" | "float" | "string" | "boolean" => {
                    enforce_arguments_count(1)?;

                    let kind = match token.raw() {
                        "integer" => Type::Integer,
                        "float" => Type::Float,
                        "string" => Type::String,
                        "boolean" => Type::Boolean,
                        _ => {
                            return Err(ErrorStack::new(
                                ErrorKind::InternalError,
                                self.0.clone(),
                                token.into(),
                            ))
                        }
                    };

                    let elem = stack.get_last().clone();

                    stack.remove_last();

                    match elem.try_cast(kind) {
                        Ok(v) => stack.push(v),
                        Err(err) => {
                            return Err(ErrorStack::with_cause(
                                ErrorKind::InternalError,
                                if let Value::String(str) = elem {
                                    ErrorStack::new(err, Rc::from(str), (0..).into())
                                } else {
                                    ErrorStack::new(err, self.0.clone(), token.into())
                                },
                                self.0.clone(),
                                token.into(),
                            ))
                        }
                    }
                }
                "eval" => {
                    enforce_arguments_count(1)?;

                    let value = stack.get_last();

                    if let Value::String(str) = value {
                        let tokens: TokenList = match str.parse() {
                            Ok(tree) => tree,
                            Err(stack) => {
                                return Err(ErrorStack::with_cause(
                                    ErrorKind::EvaluationError,
                                    stack,
                                    self.0.clone(),
                                    token.into(),
                                ))
                            }
                        };

                        let values = match tokens.run() {
                            Ok(values) => values,
                            Err(stack) => {
                                return Err(ErrorStack::with_cause(
                                    ErrorKind::EvaluationError,
                                    stack,
                                    self.0.clone(),
                                    token.into(),
                                ))
                            }
                        };

                        stack.remove_last();
                        stack.extend_from_slice(values.as_slice());
                    } else {
                        return Err(ErrorStack::new(
                            ErrorKind::InvalidType(value.kind()),
                            self.0.clone(),
                            token.into(),
                        ));
                    }
                }
                "parse" => {
                    enforce_arguments_count(1)?;

                    let value = stack.get_last();

                    if let Value::String(str) = value {
                        let value: Value = match str.parse() {
                            Ok(t) => t,
                            Err(err) => {
                                return Err(ErrorStack::with_cause(
                                    ErrorKind::EvaluationError,
                                    ErrorStack::new(err, Rc::from(str.clone()), (0..).into()),
                                    self.0.clone(),
                                    token.into(),
                                ));
                            }
                        };

                        stack.remove_last();

                        stack.push(value);
                    } else {
                        return Err(ErrorStack::new(
                            ErrorKind::InvalidType(value.kind()),
                            self.0.clone(),
                            token.into(),
                        ));
                    }
                }
                "write" => {
                    enforce_arguments_count(1)?;

                    let value = stack.get_last();

                    match stdout().write_all(
                        [value.simple_string().as_bytes(), &['\n' as u8]]
                            .concat()
                            .as_slice(),
                    ) {
                        Ok(()) => stack.remove_last(),
                        Err(err) => {
                            return Err(ErrorStack::new(
                                ErrorKind::IoError(err.kind()),
                                self.0.clone(),
                                token.into(),
                            ))
                        }
                    };
                }
                "error" => {
                    enforce_arguments_count(1)?;

                    let value = stack.get_last();

                    return Err(ErrorStack::new(
                        if let Value::String(str) = value {
                            ErrorKind::Custom(str.clone())
                        } else {
                            ErrorKind::InvalidType(value.kind())
                        },
                        self.0.clone(),
                        token.into(),
                    ));
                }
                "panic" => {
                    enforce_arguments_count(1)?;

                    let value = stack.get_last();

                    if let Value::String(str) = value {
                        panic!("{}", str);
                    } else {
                        return Err(ErrorStack::new(
                            ErrorKind::InvalidType(value.kind()),
                            self.0.clone(),
                            token.into(),
                        ));
                    }
                }
                "flat" => {
                    enforce_arguments_count(1)?;

                    let value = stack.get_last();

                    match value {
                        Value::List(v) => {
                            let mut vec: Vec<Value> = Vec::new();

                            for value in v {
                                if let Value::List(list) = value {
                                    for value2 in list {
                                        vec.push(value2.clone());
                                    }
                                } else {
                                    vec.push(value.clone());
                                }
                            }

                            stack.remove_last();
                            stack.push(vec.into());
                        }
                        e => {
                            return Err(ErrorStack::new(
                                ErrorKind::InvalidType(e.kind()),
                                self.0.clone(),
                                token.into(),
                            ))
                        }
                    }
                }
                "join" => {
                    enforce_arguments_count(1)?;

                    let value = stack.get_last();

                    match value {
                        Value::List(v) => {
                            let mut buf = String::new();

                            for value in v {
                                buf.push_str(&value.simple_string());
                            }

                            stack.remove_last();
                            stack.push(buf.into());
                        }
                        e => {
                            return Err(ErrorStack::new(
                                ErrorKind::InvalidType(e.kind()),
                                self.0.clone(),
                                token.into(),
                            ))
                        }
                    }
                }
                "chars" => {
                    enforce_arguments_count(1)?;

                    let value = stack.get_last();

                    match value {
                        Value::String(v) => {
                            let value = v
                                .chars()
                                .map(|x| Value::String(Rc::from(x.to_string().as_str())))
                                .collect::<Vec<_>>()
                                .into();

                            stack.remove_last();
                            stack.push(value);
                        }
                        e => {
                            return Err(ErrorStack::new(
                                ErrorKind::InvalidType(e.kind()),
                                self.0.clone(),
                                token.into(),
                            ))
                        }
                    }
                }
                t => stack.push(match t.parse() {
                    Ok(t) => t,
                    Err(err) => return Err(ErrorStack::new(err, self.0.clone(), token.into())),
                }),
            }
        }

        Ok(stack)
    }
}
