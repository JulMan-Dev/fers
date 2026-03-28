//! The new runtime, Fers engine

use crate::{
    parser::{
        ast::{
            ClosureExpression, Expression, ExpressionStatement, MacroStatement, MetaStatement, 
            Statement, Token, TokenMeta, VariableStatement
        },
        parse
    },
    types::{Closure, Type, Value},
    error::{ErrorKind, ErrorStack},
    frame::{Frame, FrameSecurity},
    position::ToRange,
    token::TokenList,
    utils::VecUtils
};
use std::{
    cell::RefCell,
    fmt,
    fmt::Debug,
    io::Write,
    rc::Rc
};

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
    PushClosure(ClosureExpression), // requires a frame, cannot be built on fly
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
    Call(Rc<Expression>),
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
        
        // checking for stack overflow.
        if frame.depth() > 400 {
            return Err(ErrorStack::new(
                ErrorKind::StackOverflow,
                self.source.clone(),
                (0..1).into(), // dummy range for stack overflow
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
        use BuiltOperation::*;
        
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
            PushValue(ref value) => {
                stack.push(value.clone());
            }
            PushClosure(ref closure) => {
                let closure = Closure {
                    parameters: closure.parameters.clone(),
                    body: closure.body.clone(),
                    parent: frame.clone(),
                };
                
                stack.push(Value::Closure(closure));
            }
            Add => {
                let Some([left, right]) = stack.take_last_chunk::<2>() else {
                    return Err(make_error_stack(ErrorKind::InvalidNumberArguments(2, stack.len())));
                };

                stack.push(match left + right {
                    Ok(res) => res,
                    Err(err) => return Err(make_error_stack(err)),
                });
            }
            Sub => {
                let Some([left, right]) = stack.take_last_chunk::<2>() else {
                    return Err(make_error_stack(ErrorKind::InvalidNumberArguments(2, stack.len())));
                };

                stack.push(match left - right {
                    Ok(res) => res,
                    Err(err) => return Err(make_error_stack(err)),
                });
            }
            Mul => {
                let Some([left, right]) = stack.take_last_chunk::<2>() else {
                    return Err(make_error_stack(ErrorKind::InvalidNumberArguments(2, stack.len())));
                };

                stack.push(match left * right {
                    Ok(res) => res,
                    Err(err) => return Err(make_error_stack(err)),
                });
            }
            Div => {
                let Some([left, right]) = stack.take_last_chunk::<2>() else {
                    return Err(make_error_stack(ErrorKind::InvalidNumberArguments(2, stack.len())));
                };

                stack.push(match left /right {
                    Ok(res) => res,
                    Err(err) => return Err(make_error_stack(err)),
                });
            }
            Neg => {
                let Some(e) = stack.pop() else {
                    return Err(make_error_stack(ErrorKind::InvalidNumberArguments(1, stack.len())));
                };

                stack.push(match -e {
                    Ok(res) => res,
                    Err(err) => return Err(make_error_stack(err)),
                });
            }
            Clone => {
                let Some(e) = stack.last() else {
                    return Err(make_error_stack(ErrorKind::InvalidNumberArguments(1, stack.len())));
                };

                stack.push(e.clone());
            }
            CastTo(kind) => {
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
                
                let mut child_frame = match frame.new_child(
                    frame.chunk(),
                    FrameSecurity::new().with_all_unsecured() // TODO: stricter security.
                ) {
                    Ok(child_frame) => child_frame,
                    Err(err) => return Err(make_error_stack(ErrorKind::FrameError(err))),
                };
                
                // setting caller frame
                child_frame.set_caller(Some(frame.clone()));
                
                if let Err(mut err) = self.execute_expression(&expr.expression, &mut new_stack, &child_frame, Some(str)) {
                    if err.kind() == &ErrorKind::StackOverflow {
                        err.range = token.position.to_range();
                    }
                    
                    return Err(make_error_stack_with_cause(ErrorKind::EvaluationError, err));
                }

                stack.extend(new_stack);
            }
            Parse => {
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
            Write => {
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
            Error(panicking) => {
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
                
                stack.extend(value.iter().cloned());
            } 
            Call(ref callee) => {
                // callee is a token, not a variable.
                
                let mut resolve_stack = Vec::new();
                
                match self.execute_expression(&callee, &mut resolve_stack, frame, None) {
                    Ok(()) => (),
                    Err(err) => return Err(make_error_stack_with_cause(ErrorKind::InMacroExpansion, err)),
                };

                let Some(to_call) = resolve_stack.pop() else {
                    return Err(make_error_stack(ErrorKind::InvalidNumberArguments(1, resolve_stack.len())));
                };
                stack.extend(resolve_stack); // push the rest of the stack.
                
                let to_call = match to_call {
                    Value::Closure(closure) => closure,
                    _ => return Err(make_error_stack(ErrorKind::InvalidType(to_call.kind()))),
                };
                
                let arity = to_call.parameters.len();
                
                // ensuring the arity is correct.
                if stack.len() < arity {
                    return Err(make_error_stack(ErrorKind::InvalidNumberArguments(arity, stack.len())));
                }
                
                let mut child_frame = match to_call.parent.new_child(
                    to_call.body,
                    FrameSecurity::new().with_all_unsecured()
                ) {
                    Ok(child_frame) => child_frame,
                    Err(err) => return Err(make_error_stack(ErrorKind::FrameError(err))),
                };
                
                // setting caller frame
                child_frame.set_caller(Some(frame.clone()));
                
                // push the arguments to the child frame.
                for arg in to_call.parameters.iter().rev() {
                    child_frame.push_local(
                        arg.clone(),
                        Rc::new(vec![stack.pop().expect("Cannot pop arguments from stack")]),
                    ).expect("Cannot push arguments to child frame");
                }
                
                let result = loop {
                    let result = self.step(&child_frame);

                    if let Err(ref stack) = result {
                        // Cannot continue, graceful exit
                        if matches!(stack.kind(), ErrorKind::EndOfFile) {
                            break Ok(());
                        }

                        break Err(stack.clone());
                    }
                };
                
                match result {
                    Ok(()) => (),
                    Err(mut err) => {
                        if err.kind() == &ErrorKind::StackOverflow {
                            // updates the position of the error
                            err.range = token.position.to_range();
                        }
                        
                        return Err(make_error_stack_with_cause(ErrorKind::EvaluationError, err))
                    },
                }
                
                // get "$" and push back to the stack.
                if let Ok(Some(value)) = child_frame.resolve_local("$") {
                    stack.extend(value.iter().cloned());
                }
                
                drop(child_frame); // no longer needed.
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

        for TokenMeta { token, position } in expression.iter() {
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
                Token::Closure(closure) => PushClosure(closure.clone()),
                Token::Call(call) => Call(Rc::new(call.callee.clone())),
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
