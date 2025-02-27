use std::fmt::{write, Display};

use serde::{Deserialize, Serialize};



#[derive(Debug,Clone,Serialize,Deserialize)]
pub enum Type{
    String,
    Int,
    Float,
    Bool,
    Char,
    Object(Box<ObjectType>),
}

impl Display for Type{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::String => write!(f, "string"),
            Type::Bool => write!(f, "bool"),
            Type::Char => write!(f, "char"),
            Type::Object(t) => write!(f, "{}",t),
        }
    }
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub enum ObjectType {
    Array(Type)
}

impl Display for ObjectType{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjectType::Array(typ) => write!(f, "[{}]",typ),
        }
    }
}