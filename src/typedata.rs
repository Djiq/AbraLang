use std::fmt::{write, Display};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Type {
    String,
    Int,
    Float,
    Bool,
    Char,
    Object(ObjectType),
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::String => write!(f, "string"),
            Type::Bool => write!(f, "bool"),
            Type::Char => write!(f, "char"),
            Type::Object(t) => write!(f, "{}", t),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObjectType {
    Null,
    Array(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Abra(AbraType),
}

impl Display for ObjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjectType::Abra(_) => write!(f, "[ABRA]"),
            ObjectType::Map(t1, t2) => write!(f, "<{} -> {}>", t1, t2),
            ObjectType::Null => write!(f, "<null>]"),
            ObjectType::Array(typ) => write!(f, "[{}]", typ),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbraType {}
