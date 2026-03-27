use std::{
    fmt::{self, Write},
    io,
    rc::Rc,
};

use crate::{types::Type, vm::Operation};
use crate::frame::FrameAccessError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    #[doc = "Unclosed string literal"]
    UnclosedStringLiteral,
    #[doc = "Unknown escape sequence"]
    UnknownEscapeSequence,
    #[doc = "Number or string too huge"]
    SizeOverflow,
    #[doc = "Unexpected token"]
    UnexpectedToken,
    #[doc = "Given an invalid number of arguments"]
    InvalidNumberArguments(usize, usize),
    #[doc = "Operation doesn't support a type"]
    InvalidType(Type),
    #[doc = "Internal error"]
    InternalError,
    #[doc = "Invalid type by pair"]
    InvalidTypesPair(Operation, Type, Type),
    #[doc = "Illegal cast (from type to other)."]
    IllegalCast(Type, Type),
    #[doc = "Parsing or evaluation error"]
    EvaluationError,
    #[doc = "Custom error message"]
    Custom(Rc<str>),
    #[doc = "Error in std::io"]
    IoError(io::ErrorKind),
    #[doc = "Illegal macro name (keyword or digit)"]
    IllegalMacroName,
    #[doc = "Error generated from macro"]
    InMacroExpansion,
    #[doc = "Macro depending on it-self"]
    RecursiveMacroInclusion(Vec<Rc<str>>),
    #[doc = "Redefinition of macro"]
    MacroRedefinition,
    #[doc = "End of file, this error is not a user issue"]
    EndOfFile,
    #[doc = "Unexpected end of line"]
    UnexpectedEndOfLine,
    #[doc = "Undefined variable"]
    UndefinedVariable,
    #[doc = "Meta statement is missing arguments"]
    MetaMissingArgument(usize),
    #[doc = "Unexcepted type, excepted..."]
    Excepted(Type),
    #[doc = "Frame access error"]
    FrameError(FrameAccessError),
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnclosedStringLiteral => f.write_str("Unclosed string literal")?,
            Self::UnknownEscapeSequence => f.write_str("Unknown escape sequence")?,
            Self::SizeOverflow => f.write_str(
                "Element cannot be created because it requires too much memory for it's type",
            )?,
            Self::UnexpectedToken => f.write_str("Unexpected token")?,
            Self::InvalidNumberArguments(excepted, found) => {
                write!(
                    f,
                    "Missing required arguments, excepted {excepted}, received {found}"
                )?;
            }
            Self::InvalidType(t) => write!(
                f,
                "Invalid type for operation, this operation does not support {t} type"
            )?,
            Self::InternalError => f.write_fmt(format_args!(
                "Operation failed, unexpected error: internal error"
            ))?,
            Self::InvalidTypesPair(op, a, b) => {
                write!(f, "Cannot {op} {a} and {b} types together")?
            }
            Self::IllegalCast(a, b) => write!(f, "Cannot cast {a} into a {b}")?,
            Self::EvaluationError => f.write_str("Parsing or evaluation of string failed")?,
            Self::Custom(message) => f.write_str(message)?,
            Self::IoError(why) => write!(f, "Failed to perform action: {why}")?,
            Self::IllegalMacroName => f.write_str("Illegal macro name")?,
            Self::InMacroExpansion => f.write_str("From macro expansion")?,
            Self::RecursiveMacroInclusion(set) => {
                let mut buf = format!("Recursive inclusion of {}:\n", set[0]);
                write!(buf, "  {}", set.join(" -> "))?;
                write!(f, "{}", buf.trim_end())?;
            }
            Self::MacroRedefinition => f.write_str("Redefinition of macro")?,
            Self::EndOfFile => f.write_str("End of file, this error is not a user issue")?,
            Self::UnexpectedEndOfLine => f.write_str("Unexpected end of line")?,
            Self::UndefinedVariable => f.write_str("Undefined variable")?,
            Self::MetaMissingArgument(index) => {
                write!(f, "Missing required {index} arguments {}", if *index > 1 { "s" } else { "" })?;
            }
            Self::Excepted(t) => {
                write!(f, "Excepted type {}", t)?;
            },
            Self::FrameError(e) => write!(f, "Frame error: {}", e)?,
        }

        Ok(())
    }
}
