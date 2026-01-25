//! The new runtime, Fers engine

use crate::{
    parser::{ast::{Chunk, Expression, ExpressionStatement, MacroStatement, Statement, Token, TokenMeta, VariableStatement}, parse},
    error::{ErrorKind, ErrorStack},
    token::TokenList,
    types::{Type, Value},
    utils::VecUtils,
    file::StateFile,
    parser::ast::MetaStatement,
    position::ToRange
};
use std::{
    cell::RefCell,
    collections::HashMap,
    fmt,
    io::stdout,
    io::{Stdout, Write},
    rc::Rc
};
use std::collections::HashSet;

#[derive(Clone)]
pub struct State<W>
where
    W: Write,
{
    pub source: Rc<str>,
    pub chunk: Chunk,
    pub line: usize,
    pub macros: HashMap<Rc<str>, Rc<BuiltExpression>>,
    pub variables: HashMap<Rc<str>, Vec<Value>>,
    pub writer: Rc<RefCell<W>>,
}

pub type BuiltExpression = Vec<BuiltToken>;

#[derive(Debug, Clone)]
pub struct BuiltToken {
    pub position: (usize, usize),
    pub operation: BuiltOperation,
}

#[derive(Debug, Clone)]
pub enum BuiltOperation {
    PushValue(Value),
    Add,
    Sub,
    Mul,
    Div,
    Neg,
    Clone,
    CastTo(Type),
    Eval,
    Parse,
    Write,
    Error(bool),
    Macro(Rc<str>),
    KnownMacro(Rc<str>, Rc<BuiltExpression>),
    Variable(Rc<str>),
}

pub type FastResult = Result<(), ErrorStack>;
pub type StateResult = Result<(), ErrorStack>;

impl<W> State<W>
where
    W: Write,
{
    pub fn step(&mut self) -> StateResult {
        use Statement::*;
        
        if self.chunk.statements.len() <= self.line {
            return Err(ErrorStack::new(
                ErrorKind::EndOfFile,
                self.source.clone(),
                (0..).into(),
            ));
        }

        let statement = &self.chunk.statements[self.line];
        match statement {
            Comment(_) | Blank(_) => {  },
            Meta(statement) => {
                let MetaStatement { name, arguments, range } = statement;
                
                println!("{:#?}", statement);
                
                // including new file.
                if name.as_ref() == "include" {
                    let Some(include) = arguments.get(0) else {
                        return Err(ErrorStack::new(
                            ErrorKind::MetaMissingArgument(1),
                            self.source.clone(),
                            range.clone(),
                        ));
                    };
                    
                    let Token::String(ref str) = include.token else {
                        return Err(ErrorStack::new(
                            ErrorKind::Excepted(Type::String),
                            self.source.clone(),
                            include.position.to_range(),
                        ))
                    };
                }
                
                todo!();
            }
            Macro(statement) => {
                let MacroStatement { name, expression, range } = statement;
                
                if self.macros.contains_key(name) {
                    return Err(ErrorStack::new(
                        ErrorKind::MacroRedefinition,
                        self.source.clone(),
                        range.clone(),
                    ));
                } 
                
                let compiled = self.compile_macro(&expression);
                self.macros.insert(name.clone(), compiled.into());
            }
            Variable(statement) => {
                let VariableStatement { name, expression, .. } = statement;
            
                let mut stack = Vec::new();
                self.execute_expression(&expression, &mut stack, None)?;
            
                if stack.is_empty() {
                    self.variables.remove(name);
                } else {
                    self.variables.insert(name.clone(), stack);
                }
            },
            Expression(statement) => {
                let ExpressionStatement { expression, .. } = statement;

                let mut stack = Vec::new();
                self.execute_expression(&expression, &mut stack, None)?;

                if !stack.is_empty() {
                    self.variables.insert("$".into(), stack.clone());
                }
            }
        };
        self.line += 1;
        Ok(())
    }

    /// Do expressions.
    /// 
    /// The returned stack should replace the current one.
    pub fn execute_expression(
        &self, 
        expression: &Expression,
        stack: &mut Vec<Value>,
        source: Option<Rc<str>>,
    ) -> FastResult {
        // compile expression and do on the compiled.
        let compiled = self.compile_macro(expression);
        self.execute_built_expression(Rc::new(compiled), stack, source)
    }
    
    pub fn do_token(
        &self,
        token: &BuiltToken,
        stack: &mut Vec<Value>,
        source: Option<Rc<str>>,
    ) -> FastResult {
        let make_error_stack = |kind: ErrorKind| -> ErrorStack {
            ErrorStack::new(
                kind,
                self.source.clone(),
                token.position.to_range(),
            )
        };

        let make_error_stack_with_cause = |kind: ErrorKind, stack: ErrorStack| -> ErrorStack {
            ErrorStack::with_cause(
                kind,
                stack,
                self.source.clone(),
                token.position.to_range(),
            )
        };

        match token.operation {
            BuiltOperation::PushValue(ref value) => {
                stack.push(value.clone());
            }
            BuiltOperation::Add => {
                let Some([left, right]) = stack.take_last_chunk::<2>() else {
                    return Err(make_error_stack(ErrorKind::InvalidNumberArguments(2, stack.len())));
                };

                stack.push(match left + right {
                    Ok(res) => res,
                    Err(err) => return Err(make_error_stack(err)),
                });
            }
            BuiltOperation::Sub => {
                let Some([left, right]) = stack.take_last_chunk::<2>() else {
                    return Err(make_error_stack(ErrorKind::InvalidNumberArguments(2, stack.len())));
                };

                stack.push(match left - right {
                    Ok(res) => res,
                    Err(err) => return Err(make_error_stack(err)),
                });
            }
            BuiltOperation::Mul => {
                let Some([left, right]) = stack.take_last_chunk::<2>() else {
                    return Err(make_error_stack(ErrorKind::InvalidNumberArguments(2, stack.len())));
                };

                stack.push(match left * right {
                    Ok(res) => res,
                    Err(err) => return Err(make_error_stack(err)),
                });
            }
            BuiltOperation::Div => {
                let Some([left, right]) = stack.take_last_chunk::<2>() else {
                    return Err(make_error_stack(ErrorKind::InvalidNumberArguments(2, stack.len())));
                };

                stack.push(match left /right {
                    Ok(res) => res,
                    Err(err) => return Err(make_error_stack(err)),
                });
            }
            BuiltOperation::Neg => {
                let Some(e) = stack.pop() else {
                    return Err(make_error_stack(ErrorKind::InvalidNumberArguments(1, stack.len())));
                };

                stack.push(match -e {
                    Ok(res) => res,
                    Err(err) => return Err(make_error_stack(err)),
                });
            }
            BuiltOperation::Clone => {
                let Some(e) = stack.last() else {
                    return Err(make_error_stack(ErrorKind::InvalidNumberArguments(1, stack.len())));
                };

                stack.push(e.clone());
            }
            BuiltOperation::CastTo(kind) => {
                let Some(e) = stack.pop() else {
                    return Err(make_error_stack(ErrorKind::InvalidNumberArguments(1, stack.len())));
                };

                stack.push(match e.try_cast(kind) {
                    Ok(res) => res,
                    Err(err) => return Err(make_error_stack(err)),
                });
            }
            BuiltOperation::Eval => {
                let Some(value) = stack.pop() else {
                    return Err(make_error_stack(ErrorKind::InvalidNumberArguments(1, stack.len())));
                };
                
                let Value::String(str) = value else {
                    return Err(make_error_stack(ErrorKind::InvalidType(value.kind())));
                };
                
                let tokens: TokenList = match str.parse() {
                    Ok(tree) => tree,
                    Err(stack) => {
                        return Err(make_error_stack_with_cause(
                            ErrorKind::EvaluationError,
                            stack,
                        ))
                    }
                };

                let chunk = match parse(&tokens) {
                    Ok(chunk) => chunk,
                    Err(err) => {
                        return Err(make_error_stack_with_cause(
                            ErrorKind::EvaluationError,
                            err,
                        ));
                    }
                };

                if chunk.statements.len() != 1 || !matches!(chunk.statements[0], Statement::Expression(_)) {
                    return Err(make_error_stack_with_cause(
                        ErrorKind::EvaluationError,
                        ErrorStack::new(
                            ErrorKind::UnexpectedToken,
                            str.clone(),
                            (0..).into(),
                        )
                    ));
                }

                let Statement::Expression(expr) = chunk.statements[0].clone() else {
                    unreachable!();
                };
                let mut new_stack = Vec::new();

                if let Err(err) = self.execute_expression(&expr.expression, &mut new_stack, Some(str)) {
                    return Err(make_error_stack_with_cause(ErrorKind::EvaluationError, err));
                }

                stack.extend(new_stack);
            }
            BuiltOperation::Parse => {
                let Some(value) = stack.pop() else {
                    return Err(make_error_stack(ErrorKind::InvalidNumberArguments(1, stack.len())));
                };

                let Value::String(str) = value else {
                    return Err(make_error_stack(ErrorKind::InvalidType(value.kind())));
                };
                
                let value: Value = match str.parse() {
                    Ok(t) => t,
                    Err(err) => {
                        return Err(make_error_stack_with_cause(
                            ErrorKind::EvaluationError,
                            ErrorStack::new(err, Rc::from(str.clone()), (0..).into()),
                        ));
                    }
                };

                stack.push(value);
            }
            BuiltOperation::Write => {
                let Some(value) = stack.pop() else {
                    return Err(make_error_stack(ErrorKind::InvalidNumberArguments(1, stack.len())));
                };
                
                let writer = &mut self.writer.borrow_mut();
                match  write!(writer, "{}", value.simple_string()) {
                    Ok(()) => (),
                    Err(err) => {
                        return Err(make_error_stack(ErrorKind::IoError(err.kind())))
                    }
                };
            }
            BuiltOperation::Error(panicking) => {
                let Some(value) = stack.pop() else {
                    return Err(make_error_stack(ErrorKind::InvalidNumberArguments(1, stack.len())));
                };

                let Value::String(str) = value else {
                    return Err(make_error_stack(ErrorKind::InvalidType(value.kind())));
                };
                
                if panicking {
                    panic!("{}", str);
                } else {
                    return Err(make_error_stack(ErrorKind::Custom(str.clone())));
                }
            }
            BuiltOperation::Macro(ref name) => {
                let Some(macro_definition) = self.macros.get(name) else {
                    return Err(make_error_stack(ErrorKind::UnexpectedToken));
                };
                
                self.execute_built_expression(macro_definition.clone(), stack, None).map_err(|v| { 
                    make_error_stack_with_cause(ErrorKind::InMacroExpansion, v)
                })?;
            }
            BuiltOperation::KnownMacro(_, ref operations) => {
                self.execute_built_expression(operations.clone(), stack, None).map_err(|v| {
                    make_error_stack_with_cause(ErrorKind::InMacroExpansion, v)
                })?;
            }
            BuiltOperation::Variable(ref variable) => {
                let Some(value) = self.variables.get(variable) else {
                    return Err(make_error_stack(ErrorKind::UndefinedVariable));
                };
                
                stack.extend(value.clone());
            }
        }
        
        Ok(())
    }
    
    pub fn execute_built_expression(
        &self,
        expression: Rc<BuiltExpression>,
        stack: &mut Vec<Value>,
        source: Option<Rc<str>>,
    ) -> FastResult {
        for token in expression.iter() {
            self.do_token(token, stack, source.clone())?;
        }
        
        Ok(())
    }

    pub fn compile_macro(&self, expression: &Expression) -> BuiltExpression {
        use BuiltOperation::*;

        let mut tokens = BuiltExpression::new();

        for TokenMeta { token, position } in expression {
            let operation = match token {
                Token::Identifier(identifier) => {
                    match identifier.as_ref() {
                        "+" => Add,
                        "-" => Sub,
                        "*" => Mul,
                        "/" => Div,
                        "neg" => Neg,
                        "clone" => Clone,
                        "integer" => CastTo(Type::Integer),
                        "float" => CastTo(Type::Float),
                        "string" => CastTo(Type::String),
                        "boolean" => CastTo(Type::Boolean),
                        "eval" => Eval,
                        "parse" => Parse,
                        "error" => Error(false),
                        "panic" => Error(true),
                        "write" => Write,
                        _ if identifier.starts_with("$") => {
                            Variable(identifier.clone())
                        }
                        _ => {
                            match self.macros.get(identifier) {
                                None => Macro(identifier.clone()),
                                Some(expr) => KnownMacro(identifier.clone(), expr.clone()),
                            }
                        }
                    }
                }
                Token::Null => PushValue(Value::Null),
                &Token::Integer(x) => PushValue(x.into()),
                &Token::Float(dec) => PushValue(dec.into()),
                Token::String(str) => PushValue(str.clone().into()),
                &Token::Boolean(b) => PushValue(b.into()),
            };

            tokens.push(BuiltToken { position: *position, operation });
        }

        tokens
    }
}

impl<W> fmt::Debug for State<W>
where
    W: Write,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("State")
            .field("source", &self.source)
            .field("chunk", &self.chunk)
            .field("line", &self.line)
            .field("macros", &self.macros)
            .field("writer", &"dyn std::io::Writer".to_string())
            .finish()
    }
}

impl From<Chunk> for State<Stdout> {
    fn from(chunk: Chunk) -> Self {
        Self {
            source: chunk.source.clone(),
            chunk,
            line: 0,
            macros: HashMap::new(),
            variables: HashMap::new(),
            writer: Rc::new(RefCell::new(stdout())),
        }
    }
}
