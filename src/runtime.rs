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
    GetType,
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
    pub fn step(&self, frame: &Frame) -> StateResult {
        if frame.chunk().statements.len() <= frame.pc() {
            return Err(ErrorStack::new(
                ErrorKind::EndOfFile,
                self.source.clone(),
                (0..).into(),
            ));
        }

        let statement = &frame.chunk().statements[frame.pc()];
        self.execute_statement(statement, frame)?;
        frame.set_pc(frame.pc() + 1);
        Ok(())
    }
    
    pub fn execute_statement(&self, statement: &Statement, frame: &Frame) -> FastResult {
        use Statement::*;
        
        match statement {
            Comment(_) | Blank(_) => {},
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

                // a macro already exists in the same block.
                if let Ok(Some(_)) = frame.get_macro(name) {
                    return Err(ErrorStack::new(
                        ErrorKind::MacroRedefinition,
                        self.source.clone(),
                        range.clone(),
                    ));
                }

                let compiled = self.compile_macro(&expression, frame);

                if let Err(err) = frame.push_macro(name.clone(), compiled.into()) {
                    return Err(ErrorStack::new(
                        ErrorKind::FrameError(err),
                        self.source.clone(),
                        range.clone(),
                    ));
                }
            }
            Variable(statement) => {
                let VariableStatement { name, expression, .. } = statement;

                let mut stack = Vec::new();
                self.execute_expression(&expression, &mut stack, frame, None)?;

                let result = if stack.is_empty() {
                    frame.drop_local(name)
                } else {
                    frame.push_local(name.clone(), Rc::new(stack))
                };

                if let Err(err) = result {
                    return Err(ErrorStack::new(
                        ErrorKind::FrameError(err),
                        self.source.clone(),
                        statement.range.clone(),
                    ));
                }
            },
            Statement::Expression(statement) => {
                let ExpressionStatement { expression, .. } = statement;

                let mut stack = Vec::new();
                self.execute_expression(&expression, &mut stack, frame, None)?;

                if !stack.is_empty() {
                    if let Err(err) = frame.push_local("$".into(), Rc::new(stack.clone())) {
                        return Err(ErrorStack::new(
                            ErrorKind::FrameError(err),
                            self.source.clone(),
                            statement.range.clone(),
                        ));
                    }
                }
            }
        };
        
        Ok(())
    }

    /// Do expressions.
    /// 
    /// The returned stack should replace the current one.
    pub fn execute_expression(
        &self, 
        expression: &Expression,
        stack: &mut Vec<Value>,
        frame: &Frame,
        source: Option<Rc<str>>,
    ) -> FastResult {
        // compile expression and do on the compiled.
        let compiled = self.compile_macro(expression, frame);
        self.execute_built_expression(Rc::new(compiled), stack, frame, source)
    }
    
    pub fn do_token(
        &self,
        token: &BuiltToken,
        stack: &mut Vec<Value>,
        frame: &Frame,
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
            BuiltOperation::GetType => {
                let Some(e) = stack.pop() else {
                    return Err(make_error_stack(ErrorKind::InvalidNumberArguments(1, stack.len())));
                };

                stack.push(Value::String(e.kind().to_string().into()));
            }
            Eval => {
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
                
                let child_frame = match frame.new_child(
                    frame.chunk(),
                    FrameSecurity::new().with_all_unsecured() // TODO: stricter security.
                ) {
                    Ok(child_frame) => child_frame,
                    Err(err) => return Err(make_error_stack(ErrorKind::FrameError(err))),
                };
                
                if let Err(err) = self.execute_expression(&expr.expression, &mut new_stack, &child_frame, Some(str)) {
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
            Macro(ref name) => {
                let Ok(Some(macro_definition)) = frame.resolve_macro(name.as_ref()) else {
                    return Err(make_error_stack(ErrorKind::UnexpectedToken));
                };
                
                self.execute_built_expression(macro_definition.clone(), stack, frame, None).map_err(|v| { 
                    make_error_stack_with_cause(ErrorKind::InMacroExpansion, v)
                })?;
            }
            KnownMacro(_, ref operations) => {
                self.execute_built_expression(operations.clone(), stack, frame, None).map_err(|v| {
                    make_error_stack_with_cause(ErrorKind::InMacroExpansion, v)
                })?;
            }
            Variable(ref variable) => {
                let Ok(Some(value)) = frame.resolve_local(variable.as_ref()) else {
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
        frame: &Frame,
        source: Option<Rc<str>>,
    ) -> FastResult {
        for token in expression.iter() {
            self.do_token(token, stack, frame, source.clone())?;
        }
        
        Ok(())
    }

    pub fn compile_macro(&self, expression: &Expression, frame: &Frame) -> BuiltExpression {
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
                        "type" => GetType,
                        "eval" => Eval,
                        "parse" => Parse,
                        "error" => Error(false),
                        "panic" => Error(true),
                        "write" => Write,
                        _ if identifier.starts_with("$") => {
                            Variable(identifier.clone())
                        }
                        _ => {
                            match frame.resolve_macro(identifier) {
                                Err(_) | Ok(None) => Macro(identifier.clone()),
                                Ok(Some(expr)) => KnownMacro(identifier.clone(), expr.clone()),
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
            .field("writer", &"dyn std::io::Writer".to_string())
            .finish()
    }
}
