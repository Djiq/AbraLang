

use std::fmt::format;

use serde::{Deserialize, Serialize};

use crate::{typedata::RefType, *};

macro_rules! value_implements {
    ($t:ty,$t_func:ident) => {
        impl $t for Value {
            type Output = Value;

            fn $t_func(self, rhs: Self) -> Self::Output {
                //assert_eq!(mem::discriminant(&self), mem::discriminant(&rhs));
                match (self, rhs) {
                    (Value::Integer(a), Value::Integer(b)) => Value::Integer(a.$t_func(b)),
                    (Value::Float(a), Value::Float(b)) => Value::Float(a.$t_func(b)),
                    (Value::Char(a), Value::Char(b)) => Value::Char((a as u8).$t_func(b as u8) as char),
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
                Value::Bool(x) => Ok(*x as isize as $type),
                Value::Char(x) => Ok(*x as u8 as $type),
                Value::Float(x) => Ok(*x as $type),
                Value::Integer(x) => Ok(*x as $type),
                Value::String(x) => {
                    let type_cast = x.parse();
                    if type_cast.is_err(){
                        return Err(anyhow!("Bad cast error! tried to coerce string: {} to type {}",x,stringify!($type)));
                    }
                    Ok(type_cast.unwrap())
                }
                _ => Err(anyhow!("Bad cast! expected primitive")),
            }
        }
    };
}

#[derive(Debug, Clone, Default ,Deserialize,Serialize)]
pub enum StaticValue {
    #[default]
    Null,
    Integer(isize),
    Float(f64),
    Char(char),
    Bool(bool),
    String(String),
    Object(Type,Vec<StaticValue>)
}

pub struct ObjectInitializer{
    typ: ObjectType,
    init: Vec<StaticValue>
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
            StaticValue::Object(_, _) => Value::Null,
        }
    }
    
}

impl From<Value> for StaticValue {
    fn from(value: Value) -> Self {
        match value {
            Value::String(s) => StaticValue::String(s),
            Value::Null => StaticValue::Null,
            Value::Bool(b) => StaticValue::Bool(b),
            //Value::Char(c) => StaticValue::Char(c),
            Value::Integer(i) => StaticValue::Integer(i),
            Value::Float(f) => StaticValue::Float(f),
            _ => panic!()
        }
    }

}


#[derive(Debug, Clone, Default)]
pub enum Value {
    #[default]
    Null,
    Integer(isize),
    Float(f64),
    Char(char),
    Bool(bool),
    String(String),
    Ref(Ref),
}

impl Value {

    pub fn cast(&self, to: typedata::Type) -> Result<Value>{
        match to {
            Type::Bool => Ok(Value::Bool(self.cast_to_bool()?)),
            Type::Char => Ok(Value::Char(self.cast_to_int()? as u8 as char)),
            Type::Ref(_) => Err(anyhow!("Ref is not a castable type!")),
            Type::Float => Ok(Value::Float(self.cast_to_float()?)),
            Type::Int => Ok(Value::Integer(self.cast_to_int()?)),
            Type::String => Ok(Value::String(format!("{}",&self)))
        }
    }

    pub fn get_type(&self) -> Type {
        match &self {
            Value::Bool(_) => Type::Bool,
            Value::Char(_) => Type::Char,
            Value::Float(_) => Type::Float,
            Value::Integer(_) => Type::Float,
            Value::String(_) => Type::String,
            Value::Ref(rf) => {
                rf.rf_type.clone()
            }
        }
    }

    cast_to!(cast_to_int, isize);
    cast_to!(cast_to_float, f64);
   // cast_to!(cast_to_char, char);

    pub fn cast_to_bool(&self) -> anyhow::Result<bool> {
        match self {
            
            Value::Null => Err(anyhow!("Null not expected")),
            Value::Bool(x) => Ok(*x),
            Value::Integer(x) => Ok(*x != 0),
            Value::Float(x) => Ok(*x == 0.),
            Value::Char(x) => Ok(*x as u8 == 0),
            Value::String(string) => Ok(string.len() != 0),
            Value::Ref(rf) => Ok(true),
        }
    }

    pub fn expect_null(&self) -> Result<()> {
        if matches!(self, Value::Null) {
            return Ok(());
        }
        Err(anyhow!("expected null"))
    }

    pub fn expect_int(&self) -> anyhow::Result<isize> {
        if let Value::Integer(x) = self {
            return Ok(*x);
        }
        Err(anyhow!("expected null"))
    }

    pub fn expect_float(&self) -> anyhow::Result<f64> {
        if let Value::Float(x) = self {
            return Ok(*x);
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
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(s) => write!(f,"{}",s),
            Value::Null => write!(f, ""),
            Value::Bool(x) => write!(f, "{}", x),
            Value::Integer(x) => write!(f, "{}", x),
            Value::Float(x) => write!(f, "{}", x),
            Value::Char(x) => write!(f, "{}", x),
            Value::Ref(x) => write!(f, "{}", x.id),
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

#[derive(Debug, Clone)]
pub struct Ref{
    pub rf_type: Type,
    pub id: usize,
    pub gen: usize
}

pub struct RefHeader{
    pub id: usize,
    pub gen: usize,
    pub deleted: bool,
    pub references: usize,
    pub ref_type: Mutex<RefType>,
}

#[derive(Debug, Clone)]
pub enum RefType {
    Null,
    Array(Vec<Value>),
}

impl RefType{
    pub fn get(&self,at: &Value) -> Result<Value>{
        match self {
            RefType::Null => Ok(Value::Null),
            RefType::Array(arr) => {
                let index = at.expect_int()?;
                Ok(arr[index as usize].clone())
            }
        }
    }

    pub fn modify(&mut self, at: &Value, with: Value) -> Result<()> {
        match self {
            RefType::Null => Ok(()),
            RefType::Array(arr) => {
                let index = at.expect_int()?;
                arr[index as usize] = with;
                Ok(())
            }
        }
    }
}

impl Display for RefType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RefType::Null => write!(f,"null"),
            RefType::Array(arr) => {
                let s = arr
                    .iter()
                    .map(|v| v.to_string())
                    .fold(String::new(), |acc, v| format!("{},{}", acc, v));
                write!(f, "[{}]", s)
            }
        }
    }
}