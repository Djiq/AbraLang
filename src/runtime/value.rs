use std::{
    fmt::Display,
    hash::{Hash, Hasher},
    ops::{Add, Div, Mul, Sub},
};

use serde::{Deserialize, Serialize};

use crate::{
    compiler::typecheck::{BOOL_TYPE, CHAR_TYPE, FLOAT_TYPE, INTEGER_TYPE, STRING_TYPE},
    runtime::object::Ref,
};
use anyhow::*;
use ordered_float::OrderedFloat;

use crate::compiler::typecheck::{Composite, Primitives, Type};

macro_rules! value_implements {
    ($t:ty,$t_func:ident) => {
        impl $t for Value {
            type Output = Value;

            fn $t_func(self, rhs: Self) -> Self::Output {
                //assert_eq!(mem::discriminant(&self), mem::discriminant(&rhs));
                match (self, rhs) {
                    (Value::Integer(a), Value::Integer(b)) => Value::Integer(a.$t_func(b)),
                    (Value::Float(a), Value::Float(b)) => Value::Float(a.$t_func(b)),
                    (Value::Char(a), Value::Char(b)) => {
                        Value::Char((a as u8).$t_func(b as u8) as char)
                    }
                    (_, _) => Value::Null,
                }
            }
        }
    };
}

macro_rules! cast_to {
    ($t_func:ident,$type:ty) => {
        pub fn $t_func(&self) -> anyhow::Result<$type> {
            match self {
                Value::Bool(x) => Ok(*x as i64 as $type),
                Value::Char(x) => Ok(*x as u8 as $type),
                Value::Float(x) => Ok(**x as $type),
                Value::Integer(x) => Ok(*x as $type),
                Value::String(x) => {
                    let type_cast = x.parse();
                    if type_cast.is_err() {
                        return Err(anyhow!(
                            "Bad cast error! tried to coerce string: {} to type {}",
                            x,
                            stringify!($type)
                        ));
                    }
                    Ok(type_cast.unwrap())
                }
                _ => Err(anyhow!("Bad cast! expected primitive")),
            }
        }
    };
}

#[derive(Debug, Clone, Default, PartialEq, PartialOrd, Deserialize, Serialize, Eq)]
pub enum StaticValue {
    #[default]
    Null,
    Integer(i64),
    Float(OrderedFloat<f64>),
    Char(char),
    Bool(bool),
    String(String),
}

impl Display for StaticValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StaticValue::String(s) => write!(f, "{}", s),
            StaticValue::Null => write!(f, ""),
            StaticValue::Bool(x) => write!(f, "{}", x),
            StaticValue::Integer(x) => write!(f, "{}", x),
            StaticValue::Float(x) => write!(f, "{}", x),
            StaticValue::Char(x) => write!(f, "{}", x),
        }
    }
}

impl From<i64> for StaticValue {
    fn from(value: i64) -> Self {
        StaticValue::Integer(value)
    }
}

impl From<f64> for StaticValue {
    fn from(value: f64) -> Self {
        StaticValue::Float(OrderedFloat(value))
    }
}

impl From<char> for StaticValue {
    fn from(value: char) -> Self {
        StaticValue::Char(value)
    }
}

impl From<bool> for StaticValue {
    fn from(value: bool) -> Self {
        StaticValue::Bool(value)
    }
}

impl From<String> for StaticValue {
    fn from(value: String) -> Self {
        StaticValue::String(value)
    }
}

impl TryInto<i64> for StaticValue {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<i64, Self::Error> {
        match self {
            StaticValue::Integer(x) => Ok(x),
            x => bail!("{x:?} cannot be converted to i64"),
        }
    }
}

impl TryInto<f64> for StaticValue {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<f64, Self::Error> {
        match self {
            StaticValue::Float(x) => Ok(*x),
            x => bail!("{x:?} cannot be converted to f64"),
        }
    }
}

impl TryInto<char> for StaticValue {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<char, Self::Error> {
        match self {
            StaticValue::Char(x) => Ok(x),
            x => bail!("{x:?} cannot be converted to char"),
        }
    }
}

impl TryInto<bool> for StaticValue {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<bool, Self::Error> {
        match self {
            StaticValue::Bool(x) => Ok(x),
            x => bail!("{x:?} cannot be converted to bool"),
        }
    }
}

impl TryInto<String> for StaticValue {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<String, Self::Error> {
        match self {
            StaticValue::String(x) => Ok(x),
            x => bail!("{x:?} cannot be converted to String"),
        }
    }
}

impl Into<Value> for StaticValue {
    fn into(self) -> Value {
        match self {
            StaticValue::String(string) => Value::String(string),
            StaticValue::Null => Value::Null,
            StaticValue::Bool(b) => Value::Bool(b),
            StaticValue::Char(c) => Value::Char(c),
            StaticValue::Integer(i) => Value::Integer(i),
            StaticValue::Float(f) => Value::Float(f),
            //StaticValue::Object(_) => Value::Null,
        }
    }
}

impl TryFrom<Value> for StaticValue {
    type Error = anyhow::Error;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        Ok(match value {
            Value::String(s) => StaticValue::String(s),
            Value::Null => StaticValue::Null,
            Value::Bool(b) => StaticValue::Bool(b),
            Value::Char(c) => StaticValue::Char(c),
            Value::Integer(i) => StaticValue::Integer(i),
            Value::Float(f) => StaticValue::Float(f),
            x => bail!("{x:?} cannot be converted to StaticValue"),
        })
    }
}

#[derive(Debug, Clone, Default, Eq)]
pub enum Value {
    #[default]
    Null,
    Integer(i64),
    Float(OrderedFloat<f64>),
    Char(char),
    Bool(bool),
    String(String),
    Ref(Ref),
}

impl TryInto<i64> for Value {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<i64, Self::Error> {
        match self {
            Value::Integer(x) => Ok(x),
            x => bail!("{x:?} cannot be converted to i64"),
        }
    }
}

impl TryInto<f64> for Value {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<f64, Self::Error> {
        match self {
            Value::Float(x) => Ok(x.into_inner()),
            x => bail!("{x:?} cannot be converted to f64"),
        }
    }
}

impl TryInto<char> for Value {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<char, Self::Error> {
        match self {
            Value::Char(x) => Ok(x),
            x => bail!("{x:?} cannot be converted to char"),
        }
    }
}

impl TryInto<bool> for Value {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<bool, Self::Error> {
        match self {
            Value::Bool(x) => Ok(x),
            x => bail!("{x:?} cannot be converted to bool"),
        }
    }
}

impl TryInto<String> for Value {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<String, Self::Error> {
        match self {
            Value::String(x) => Ok(x),
            x => bail!("{x:?} cannot be converted to String"),
        }
    }
}

impl TryInto<Ref> for Value {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Ref, Self::Error> {
        match self {
            Value::Ref(x) => Ok(x),
            x => bail!("{x:?} cannot be converted to Ref"),
        }
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Value::Integer(value)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Value::Float(OrderedFloat(value))
    }
}

impl From<char> for Value {
    fn from(value: char) -> Self {
        Value::Char(value)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Bool(value)
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::String(value)
    }
}

impl<'a> From<&'a str> for Value {
    fn from(value: &'a str) -> Self {
        Value::String(value.into())
    }
}

const FLOAT_PRECISION_HASH: u64 = 256;

//impl Eq for Value {}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Value::Null => 0.hash(state), // Use a constant discriminant for Null
            Value::Integer(i) => {
                1.hash(state); // Discriminant for Integer
                i.hash(state);
            }
            Value::Float(f) => {
                2.hash(state); // Discriminant for Float
                let v = (f * FLOAT_PRECISION_HASH as f64).floor() as u64;
                v.hash(state);
            }
            Value::Char(c) => {
                3.hash(state); // Discriminant for Char
                c.hash(state);
            }
            Value::Bool(b) => {
                4.hash(state); // Discriminant for Bool
                b.hash(state);
            }
            Value::String(s) => {
                5.hash(state); // Discriminant for String
                s.hash(state);
            }
            Value::Ref(r) => {
                6.hash(state); // Discriminant for Ref
                r.get_uuid().hash(state); // Hash the unique identifier of the reference
            }
        }
    }
}

impl From<Type> for Value {
    fn from(value: Type) -> Self {
        match value {
            Type::Primitive(p) => match p {
                        Primitives::String => "".into(),
                        Primitives::Integer => 0.into(),
                        Primitives::Float => 0.0.into(),
                        Primitives::Bool => false.into(),
                        Primitives::Char => '\0'.into(),
                    },
            Type::Composite(_) => panic!("Cannot create default Value from Composite type directly. Instantiate a Ref instead."),
            Type::Abra(_) => panic!("Cannot create default Value from Abra type directly. Instantiate a Ref instead."),
            Type::Algebraic(_) => panic!("Cannot create default Value from Algebraic type directly. Instantiate a Ref instead."),
            Type::Null => Value::Null,
        }
    }
}

impl Into<Type> for Value {
    fn into(self) -> Type {
        self.get_type()
    }
}

impl Value {
    pub fn get_string_representation(&self) -> String {
        match self {
            Value::Null => "null".into(),
            Value::Bool(x) => format!("{}", x),
            Value::Char(x) => format!("{}", x),
            Value::Float(x) => format!("{}", x),
            Value::Integer(x) => format!("{}", x),
            Value::String(x) => format!("{}", x),
            Value::Ref(x) => format!("Ref<{}>", x.get_uuid()),
        }
    }

    pub fn cast(&self, to: Type) -> Result<Value> {
        match to {
            Type::Primitive(p) => match p {
                Primitives::Bool => Ok(Value::Bool(self.cast_to_bool()?)),
                Primitives::Char => Ok(Value::Char(self.cast_to_int()? as u8 as char)),
                Primitives::Integer => Ok(Value::Integer(self.cast_to_int()?)),
                Primitives::Float => Ok(self.cast_to_float()?.into()),
                Primitives::String => Ok(Value::String(format!("{}", &self))),
            },
            Type::Composite(_) => Err(anyhow!("Cannot cast to a composite type directly.")),
            Type::Abra(_) => Err(anyhow!("Cannot cast to an Abra type directly.")),
            Type::Null => Err(anyhow!("Cannot cast to a null type directly.")),
            Type::Algebraic(_) => Err(anyhow!("Cannot cast to an algebraic type directly.")),
        }
    }

    pub fn cast_to_float(&self) -> anyhow::Result<f64> {
        match self {
            Value::Bool(x) => Ok(*x as i64 as f64),
            Value::Char(x) => Ok(*x as u8 as f64),
            Value::Float(x) => Ok(**x),
            Value::Integer(x) => Ok(*x as f64),
            Value::String(x) => {
                let type_cast = x.parse();
                if type_cast.is_err() {
                    return Err(anyhow!(
                        "Bad cast error! tried to coerce string: {} to type {}",
                        x,
                        "f64"
                    ));
                }
                Ok(type_cast.unwrap())
            }
            _ => Err(anyhow!("Bad cast! expected primitive")),
        }
    }

    pub fn get_type(&self) -> Type {
        match &self {
            Value::Null => Type::Null,
            Value::Bool(_) => Type::Primitive(Primitives::Bool),
            Value::Char(_) => Type::Primitive(Primitives::Char),
            Value::Float(_) => Type::Primitive(Primitives::Float),
            Value::Integer(_) => Type::Primitive(Primitives::Integer), // Corrected from old system's Type::Float
            Value::String(_) => Type::Primitive(Primitives::String),
            Value::Ref(rf) => {
                // This will call the updated Ref::get_type which returns the new compiler::typecheck::Type
                rf.get_type()
            }
        }
    }

    cast_to!(cast_to_int, i64);
    //cast_to!(cast_to_float, OrderedFloat<f64>);
    // cast_to!(cast_to_char, char);

    pub fn cast_to_bool(&self) -> anyhow::Result<bool> {
        match self {
            Value::Null => Err(anyhow!("Null not expected")),
            Value::Bool(x) => Ok(*x),
            Value::Integer(x) => Ok(*x != 0),
            Value::Float(x) => Ok(*x == 0.),
            Value::Char(x) => Ok(*x as u8 == 0),
            Value::String(string) => Ok(string.len() != 0),
            Value::Ref(rf) => Ok(rf.is_null()),
        }
    }

    pub fn expect_null(&self) -> Result<()> {
        if matches!(self, Value::Null) {
            return Ok(());
        }
        Err(anyhow!("expected null"))
    }

    pub fn expect_int(&self) -> anyhow::Result<i64> {
        if let Value::Integer(x) = self {
            return Ok(*x);
        }
        Err(anyhow!("expected null"))
    }

    pub fn expect_float(&self) -> anyhow::Result<f64> {
        if let Value::Float(x) = self {
            return Ok(**x);
        }
        Err(anyhow!("expected null"))
    }

    pub fn expect_bool(&self) -> anyhow::Result<bool> {
        if let Value::Bool(x) = self {
            return Ok(*x);
        }
        Err(anyhow!("expected null"))
    }

    pub fn expect_char(&self) -> anyhow::Result<char> {
        if let Value::Char(x) = self {
            return Ok(*x);
        }
        Err(anyhow!("expected null"))
    }

    pub fn expect_ref(&self) -> anyhow::Result<Ref> {
        if let Value::Ref(x) = self {
            return Ok(x.clone());
        }
        Err(anyhow!("expected ref"))
    }

    pub fn expect_ref_extract(&self) -> anyhow::Result<&Ref> {
        if let Value::Ref(x) = self {
            return Ok(x);
        }
        Err(anyhow!("expected ref"))
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(s) => write!(f, "{}", s),
            Value::Null => write!(f, ""),
            Value::Bool(x) => write!(f, "{}", x),
            Value::Integer(x) => write!(f, "{}", x),
            Value::Float(x) => write!(f, "{}", x),
            Value::Char(x) => write!(f, "{}", x),
            Value::Ref(x) => write!(f, "{}", x),
        }
    }
}

value_implements!(Add, add);
value_implements!(Mul, mul);
value_implements!(Sub, sub);
value_implements!(Div, div);

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Char(a), Value::Char(b)) => a == b,
            (_, _) => false,
        }
    }
    fn ne(&self, other: &Self) -> bool {
        !(self == other)
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self == other {
            return Some(std::cmp::Ordering::Equal);
        }
        if self < other {
            return Some(std::cmp::Ordering::Less);
        }
        if self > other {
            return Some(std::cmp::Ordering::Greater);
        }
        None
    }

    fn gt(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Bool(a), Value::Bool(b)) => a > b,
            (Value::Integer(a), Value::Integer(b)) => a > b,
            (Value::Float(a), Value::Float(b)) => a > b,
            (Value::Char(a), Value::Char(b)) => a > b,
            (_, _) => false,
        }
    }

    fn lt(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Bool(a), Value::Bool(b)) => a < b,
            (Value::Integer(a), Value::Integer(b)) => a < b,
            (Value::Float(a), Value::Float(b)) => a < b,
            (Value::Char(a), Value::Char(b)) => a < b,
            (_, _) => false,
        }
    }

    fn ge(&self, other: &Self) -> bool {
        self >= other
    }

    fn le(&self, other: &Self) -> bool {
        self <= other
    }
}
