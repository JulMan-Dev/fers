//! Manage all types operations (add, mul...)

use std::{
    fmt::Write,
    ops::{Add, Div, Mul, Neg, Not, Sub},
};

use crate::{
    error::ErrorKind,
    types::{Type, Value},
    utils::integer_to_float,
    vm::Operation,
};

impl Add for Value {
    type Output = Result<Value, ErrorKind>;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (_, Value::Null) | (Value::Null, _) => Ok(Value::Null),
            (Value::List(mut vec), after) => {
                vec.push(after);

                return Ok(Value::List(vec));
            }
            (before, Value::List(mut vec)) => {
                vec.insert(0, before);

                return Ok(Value::List(vec));
            }
            (Value::Integer(a), Value::Integer(b)) => match a.checked_add(b) {
                Some(res) => Ok(res.into()),
                None => Ok(Value::Null), // Err(ErrorKind::SizeOverflow),
            },
            (Value::Float(a), Value::Float(b)) => match a.checked_add(b) {
                Some(res) => Ok(res.into()),
                None => Ok(Value::Null), // Err(ErrorKind::SizeOverflow),
            },
            (Value::Integer(a), Value::Float(b)) => {
                let a = match integer_to_float(a) {
                    Ok(v) => v,
                    Err(err) => return Err(err),
                };

                match a.checked_add(b) {
                    Some(res) => Ok(res.into()),
                    None => Ok(Value::Null), // Err(ErrorKind::SizeOverflow),
                }
            }
            (Value::Float(a), Value::Integer(b)) => {
                let b = match integer_to_float(b) {
                    Ok(v) => v,
                    Err(err) => return Err(err),
                };

                match a.checked_add(b) {
                    Some(res) => Ok(res.into()),
                    None => Ok(Value::Null), // Err(ErrorKind::SizeOverflow),
                }
            }
            (s, Value::String(v)) => {
                let left = s.simple_string();

                Ok((left + v.as_ref()).into())
            }
            (Value::String(v), s) => {
                let mut buffer = v.to_string();

                match buffer.write_str(&s.simple_string()) {
                    Ok(_) => Ok(buffer.into()),
                    Err(_) => Err(ErrorKind::InternalError),
                }
            }
            (_, Value::Boolean(_)) | (Value::Boolean(_), _) => {
                Err(ErrorKind::InvalidType(Type::Boolean))
            }
        }
    }
}

impl Sub for Value {
    type Output = Result<Value, ErrorKind>;

    fn sub(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (_, Value::Null) | (Value::Null, _) => Ok(Value::Null),
            (prec, Value::List(vec)) | (Value::List(vec), prec) => {
                let mut vec = vec.clone();
                let index = vec.iter().position(|x| *x == prec);

                if let Some(index) = index {
                    vec.remove(index);
                }

                Ok(vec.into())
            }
            (Value::Integer(a), Value::Integer(b)) => match a.checked_sub(b) {
                Some(res) => Ok(res.into()),
                None => Ok(Value::Null), // Err(ErrorKind::SizeOverflow),
            },
            (Value::Float(a), Value::Float(b)) => match a.checked_sub(b) {
                Some(res) => Ok(res.into()),
                None => Ok(Value::Null), // Err(ErrorKind::SizeOverflow),
            },
            (Value::Integer(a), Value::Float(b)) => {
                let a = match integer_to_float(a) {
                    Ok(v) => v,
                    Err(err) => return Err(err),
                };

                match a.checked_sub(b) {
                    Some(res) => Ok(res.into()),
                    None => Ok(Value::Null), // Err(ErrorKind::SizeOverflow),
                }
            }
            (Value::Float(a), Value::Integer(b)) => {
                let b = match integer_to_float(b) {
                    Ok(v) => v,
                    Err(err) => return Err(err),
                };

                match a.checked_sub(b) {
                    Some(res) => Ok(res.into()),
                    None => Ok(Value::Null), // Err(ErrorKind::SizeOverflow),
                }
            }
            (Value::String(_), _) | (_, Value::String(_)) => {
                Err(ErrorKind::InvalidType(Type::String))
            }
            (Value::Boolean(_), _) | (_, Value::Boolean(_)) => {
                Err(ErrorKind::InvalidType(Type::Boolean))
            }
        }
    }
}

impl Mul for Value {
    type Output = Result<Value, ErrorKind>;

    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (_, Value::Null) | (Value::Null, _) => Ok(Value::Null),
            (Value::Integer(a), Value::Integer(b)) => match a.checked_mul(b) {
                Some(res) => Ok(res.into()),
                None => Ok(Value::Null), // Err(ErrorKind::SizeOverflow),
            },
            (Value::Integer(i), Value::Float(f)) | (Value::Float(f), Value::Integer(i)) => {
                let f1 = match integer_to_float(i) {
                    Ok(v) => v,
                    Err(err) => return Err(err),
                };

                match f.checked_mul(f1) {
                    Some(res) => Ok(res.into()),
                    None => Ok(Value::Null), // Err(ErrorKind::SizeOverflow),
                }
            }
            (Value::Float(a), Value::Float(b)) => match a.checked_mul(b) {
                Some(res) => Ok(res.into()),
                None => Ok(Value::Null), // Err(ErrorKind::SizeOverflow),
            },
            (Value::String(s), Value::Integer(c)) | (Value::Integer(c), Value::String(s)) => {
                if c.is_negative() || c > (usize::MAX / 1e12 as usize) as i128 {
                    return Ok(Value::Null); // Err(ErrorKind::SizeOverflow);
                }

                match s.len().checked_mul(c as usize) {
                    Some(s) => s,
                    None => return Ok(Value::Null), // Err(ErrorKind::SizeOverflow),
                };

                Ok(s.repeat(c as usize).into())
            }
            (Value::String(_), v) | (v, Value::String(_)) => Err(ErrorKind::InvalidTypesPair(
                Operation::Mul,
                Type::String,
                v.kind(),
            )),
            (Value::Boolean(_), _) | (_, Value::Boolean(_)) => {
                Err(ErrorKind::InvalidType(Type::Boolean))
            }
            (Value::Integer(n), Value::List(vec)) | (Value::List(vec), Value::Integer(n)) => {
                if n.is_negative() || n > (usize::MAX / 1e12 as usize) as i128 {
                    return Ok(Value::Null); // Err(ErrorKind::SizeOverflow);
                }

                match vec.len().checked_mul(n as usize) {
                    Some(s) => s,
                    None => return Ok(Value::Null), // Err(ErrorKind::SizeOverflow),
                };

                let mut output = Vec::with_capacity(vec.len() * (n as usize));

                for i in 0..output.capacity() {
                    output.push(vec[i % vec.len()].clone());
                }

                Ok(output.into())
            }
            (Value::List(_), Value::Float(_)) | (Value::Float(_), Value::List(_)) => Err(
                ErrorKind::InvalidTypesPair(Operation::Mul, Type::List, Type::Float),
            ),
            (Value::List(_), Value::List(_)) => Err(ErrorKind::InvalidTypesPair(
                Operation::Mul,
                Type::List,
                Type::List,
            )),
        }
    }
}

impl Div for Value {
    type Output = Result<Value, ErrorKind>;

    fn div(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (_, Value::Null) | (Value::Null, _) => Ok(Value::Null),
            (Value::Integer(a), Value::Integer(b)) => {
                let a = match integer_to_float(a) {
                    Ok(v) => v,
                    Err(err) => return Err(err),
                };
                let b = match integer_to_float(b) {
                    Ok(v) => v,
                    Err(err) => return Err(err),
                };

                match a.checked_div(b) {
                    Some(res) => Ok(res.into()),
                    None => Ok(Value::Null), // Err(ErrorKind::SizeOverflow),
                }
            }
            (Value::Float(a), Value::Float(b)) => match a.checked_div(b) {
                Some(res) => Ok(res.into()),
                None => Ok(Value::Null), // Err(ErrorKind::SizeOverflow),
            },
            (Value::Integer(a), Value::Float(b)) => {
                let a = match integer_to_float(a) {
                    Ok(v) => v,
                    Err(err) => return Err(err),
                };

                match a.checked_div(b) {
                    Some(res) => Ok(res.into()),
                    None => Ok(Value::Null), // Err(ErrorKind::SizeOverflow),
                }
            }
            (Value::Float(a), Value::Integer(b)) => {
                let b = match integer_to_float(b) {
                    Ok(v) => v,
                    Err(err) => return Err(err),
                };

                match a.checked_div(b) {
                    Some(res) => Ok(res.into()),
                    None => Ok(Value::Null), // Err(ErrorKind::SizeOverflow),
                }
            }
            (Value::String(_), _) | (_, Value::String(_)) => {
                return Err(ErrorKind::InvalidType(Type::String))
            }
            (Value::Boolean(_), _) | (_, Value::Boolean(_)) => {
                return Err(ErrorKind::InvalidType(Type::Boolean))
            }
            (Value::List(_), v) | (v, Value::List(_)) => Err(ErrorKind::InvalidTypesPair(
                Operation::Div,
                Type::List,
                v.kind(),
            )),
        }
    }
}

impl Neg for Value {
    type Output = Result<Value, ErrorKind>;

    fn neg(self) -> Self::Output {
        match self {
            Value::Null => Ok(Value::Null),
            Value::Integer(i) => Ok(i.neg().into()),
            Value::Float(f) => Ok(f.neg().into()),
            Value::String(s) => Ok(s.chars().rev().collect::<String>().into()),
            Value::Boolean(b) => Ok(b.not().into()),
            Value::List(v) => Ok(v
                .iter()
                .rev()
                .map(|r| r.clone())
                .collect::<Vec<Value>>()
                .into()),
        }
    }
}
