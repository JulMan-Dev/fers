//! Store all types and conversions for them.

use std::{
    fmt::{Debug, Display}, ops::Not, rc::Rc, str::FromStr
};

use rust_decimal::{prelude::Zero, Decimal, Error};

use crate::{
    frame::Frame,
    parser::ast::Chunk,
    error::ErrorKind,
    tty::{
        ansi::{NoColor, Style, RESET},
        rgb::RgbColor,
        simple::Color4,
    },
    utils::integer_to_float,
    ITALIC
};

pub type INTEGER = i128;
pub type FLOAT = Decimal;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Type {
    Null,
    Integer,
    Float,
    String,
    Boolean,
    List,
    Closure,
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Type::Null => "null",
            Type::Integer => "integer",
            Type::Float => "float",
            Type::String => "string",
            Type::Boolean => "boolean",
            Type::List => "list",
            Type::Closure => "closure",
        })
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    /// Represents a `null` value, no value.
    Null,
    /// Represents a signed integer coded in 128 bits.
    Integer(i128),
    /// Represnets a signed float.
    Float(Decimal),
    /// Represents a string.
    String(Rc<str>),
    /// Represents a boolean.
    Boolean(bool),
    /// Represents a list of values.
    List(Vec<Value>),
    /// Represents a closure.
    Closure(Closure),
}

#[derive(Debug, Clone)]
pub struct Closure {
    pub parameters: Rc<[Rc<str>]>,
    pub body: Rc<Chunk>,
    pub parent: Frame,
}

impl Value {
    /// Returns the type of the stored value.
    pub fn kind(&self) -> Type {
        match self {
            Value::Null => Type::Null,
            Value::Integer(_) => Type::Integer,
            Value::Float(_) => Type::Float,
            Value::String(_) => Type::String,
            Value::Boolean(_) => Type::Boolean,
            Value::List(_) => Type::List,
            Value::Closure(_) => Type::Closure,
        }
    }

    /// Returns the string representation of value.
    pub fn simple_string(&self) -> String {
        match self {
            Value::Null => "null".into(),
            Value::String(str) => str.to_string(),
            Value::Float(f) => {
                if f.floor() == *f {
                    let mut str = f.to_string();

                    if !str.contains('.') {
                        str += ".0";
                    }

                    str
                } else {
                    f.to_string()
                }
            }
            Value::Integer(i) => i.to_string(),
            Value::Boolean(b) => b.to_string(),
            Value::List(l) => l
                .iter()
                .map(|ref value| value.simple_string())
                .collect::<Vec<String>>()
                .join(", "),
            Value::Closure(_) => "<closure>".into(),
        }
    }

    /// Try casting a value into an other type.
    pub fn try_cast(&self, kind: Type) -> Result<Value, ErrorKind> {
        match (self, kind) {
            (_, Type::Null) => Ok(Value::Null),
            (Value::Closure(_), Type::Closure) => Ok(self.clone()),
            (Value::Closure(_), _) => Ok(Value::Null), // cannot cast to anything but closure.
            (_, Type::Closure) => Err(ErrorKind::IllegalCast(Type::Closure, kind)),
            (Value::List(_), Type::List) => Ok(self.clone()),
            (_, Type::List) => Ok(Value::List(vec![self.clone()])),
            (Value::List(_), _) => Err(ErrorKind::IllegalCast(Type::List, kind)),

            (Value::Integer(_), Type::Integer) => Ok(self.clone()),
            (Value::Float(_), Type::Float) => Ok(self.clone()),
            (Value::String(_), Type::String) => Ok(self.clone()),
            (Value::Boolean(_), Type::Boolean) => Ok(self.clone()),

            (v, Type::String) => Ok(v.simple_string().into()),
            
            (Value::Integer(n), Type::Float) => Ok(integer_to_float(*n)?.into()),
            (Value::Integer(n), Type::Boolean) => Ok(n.is_zero().not().into()),
            (Value::Float(d), Type::Integer) => Ok(d.round().normalize().mantissa().into()),
            (Value::Float(d), Type::Boolean) => Ok(d.is_zero().not().into()),
            (Value::String(s), Type::Integer) => match s.parse::<INTEGER>() {
                Ok(v) => Ok(v.into()),
                Err(_) => Err(ErrorKind::UnexpectedToken),
            },
            (Value::String(s), Type::Float) => match s.parse::<FLOAT>() {
                Ok(v) => Ok(v.into()),
                Err(_) => Err(ErrorKind::UnexpectedToken),
            },
            (Value::String(s), Type::Boolean) => Ok(s.len().is_zero().not().into()),
            (Value::Boolean(b), Type::Integer) => Ok((*b as i128).into()),
            (Value::Boolean(b), Type::Float) => {
                Ok(Decimal::from_i128_with_scale(*b as i128, 0).into())
            }
            (Value::Null, Type::Integer) => Ok(0.into()),
            (Value::Null, Type::Float) => Ok(Decimal::ZERO.into()),
            (Value::Null, Type::Boolean) => Ok(false.into()),
        }
    }
}

pub const NULL_STYLE: Style<Color4, NoColor> =
    Style::empty().with_italic().with_cyan_foreground(false);
pub const STRING_STYLE: Style<Color4, NoColor> = Style::empty().with_yellow_foreground(true);
pub const NUMBER_STYLE: Style<Color4, NoColor> = Style::empty().with_blue_foreground(false);
pub const TRUE_STYLE: Style<Color4, NoColor> =
    Style::empty().with_green_foreground(false).with_bold();
pub const FALSE_STYLE: Style<Color4, NoColor> =
    Style::empty().with_red_foreground(false).with_bold();

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&match self {
            Value::Null => NULL_STYLE.apply_to("null".into()),
            Value::String(str) => {
                if str.len() > 200 {
                    let chars: Vec<String> = str.chars().map(|ch| ch.to_string()).collect();

                    let mut buf = String::new();
                    buf.push_str(&STRING_STYLE.get_sequence());
                    buf.push('"');
                    buf.push_str(&chars[0..10].concat());
                    buf.push_str(&ITALIC.get_sequence());
                    buf.push_str(&format!("...<{} more>...", chars.len() - 20));
                    buf.push_str(&STRING_STYLE.get_sequence());
                    buf.push_str(&format!("{}\"", chars[chars.len() - 10..].concat()));
                    buf.push_str(RESET);
                    buf
                } else {
                    let mut buf = String::new();
                    buf.push_str(&STRING_STYLE.get_sequence());
                    buf.push('"');

                    if str.as_ref() == "i like RAINBOW!!" {
                        buf.push_str(&RgbColor::apply_rainbow(
                            &str.escape_default().to_string(),
                            false,
                            false,
                        ));
                        buf.push_str(&STRING_STYLE.get_sequence());
                    } else {
                        buf.push_str(&format!("\"{}\"", str.escape_default()));
                    }

                    buf.push('"');
                    buf.push_str(RESET);
                    buf
                }
            }
            Value::Float(f) => NUMBER_STYLE.apply_to(&{
                if f.floor() == *f {
                    let mut str = f.to_string();
                    if !str.contains('.') {
                        str += ".0";
                    }
                    str
                } else {
                    f.to_string()
                }
            }),
            Value::Integer(i) => NUMBER_STYLE.apply_to(&i.to_string()),
            Value::Boolean(b) => {
                (if *b { TRUE_STYLE } else { FALSE_STYLE }).apply_to(&b.to_string())
            }
            Value::List(vec) => {
                let mut out = String::from('[');

                for (index, value) in vec.iter().enumerate() {
                    out.push_str(&format!("{value}"));

                    if vec.len() - 1 != index {
                        out.push_str(", ");
                    }
                }

                out.push(']');
                out
            }
        })
    }
}

impl FromStr for Value {
    type Err = ErrorKind;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match (s.starts_with('"'), s.ends_with('"')) {
            (true, true) => {
                let str: String = s[1..s.len() - 1].into();

                Ok(str.into())
            }
            (true, false) | (false, true) => unreachable!(),
            (false, false) => {
                if s == "null" {
                    Ok(Value::Null)
                } else if let Ok(b) = s.parse() {
                    Ok(Value::Boolean(b))
                } else if let Ok(i) = s.parse() {
                    Ok(Value::Integer(i))
                } else if let Ok(f) = s.parse::<Decimal>() {
                    Ok(Value::Float(f.normalize()))
                } else {
                    Err(ErrorKind::UnexpectedToken)
                }
            }
        }
    }
}

impl From<i128> for Value {
    fn from(v: i128) -> Self {
        Self::Integer(v)
    }
}

impl From<Decimal> for Value {
    fn from(v: Decimal) -> Self {
        Self::Float(v.normalize())
    }
}

impl From<Rc<str>> for Value {
    fn from(value: Rc<str>) -> Self {
        Self::String(value)
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Self::String(Rc::from(v))
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Self::Boolean(v)
    }
}

impl From<Vec<Value>> for Value {
    fn from(v: Vec<Value>) -> Self {
        Self::List(v)
    }
}

impl Value {
    /// Returns `true` if the value is [`Null`].
    ///
    /// [`Null`]: Value::Null
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Returns `true` if the value is [`Integer`].
    ///
    /// [`Integer`]: Value::Integer
    #[must_use]
    pub fn is_integer(&self) -> bool {
        matches!(self, Self::Integer(..))
    }

    /// Returns `true` if the value is [`Float`].
    ///
    /// [`Float`]: Value::Float
    #[must_use]
    pub fn is_float(&self) -> bool {
        matches!(self, Self::Float(..))
    }

    /// Returns `true` if the value is [`String`].
    ///
    /// [`String`]: Value::String
    #[must_use]
    pub fn is_string(&self) -> bool {
        matches!(self, Self::String(..))
    }

    /// Returns `true` if the value is [`Boolean`].
    ///
    /// [`Boolean`]: Value::Boolean
    #[must_use]
    pub fn is_boolean(&self) -> bool {
        matches!(self, Self::Boolean(..))
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Integer(l0), Self::Integer(r0)) => l0 == r0,
            (Self::Float(l0), Self::Float(r0)) => l0 == r0,
            (Self::String(l0), Self::String(r0)) => l0 == r0,
            (Self::Boolean(l0), Self::Boolean(r0)) => l0 == r0,
            (Self::List(l0), Self::List(r0)) => l0 == r0,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}
