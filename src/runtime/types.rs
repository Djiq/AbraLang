use std::{collections::HashMap, fmt::Display};

use serde::{Deserialize, Serialize};
use crate::runtime::value::Value;

use crate::runtime::object::RefObject;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum Type {
    String,
    Int,
    Float,
    Bool,
    Char,
    Object(ObjectType),
}

impl Type {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum ObjectType {
    Null,
    BoxedValue,
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
            ObjectType::BoxedValue => todo!(),
        }
    }
}

impl ObjectType{
    pub fn instance_self(&self,init: Vec<Value>) -> RefObject {
        match &self {
            ObjectType::Abra(_) => {
                        todo!()
                    }
            ObjectType::Map(t1, t2) => {
                        let mut map = HashMap::new();
                        let objects = init.len() / 2;
                        let init_clone = init.clone();
                        for x in 0..objects {
                            let key: Value = init_clone[2 * x].clone().into();
                            let value = &init_clone[2 * x + 1];
                            map.insert(key.get_string_representation(), value.clone().into());
                        }

                        RefObject::Map(*t1.clone(), *t2.clone(), map)
                    }
            ObjectType::Null => panic!(),
            ObjectType::Array(typ) => RefObject::Array(
                        *typ.clone(),
                        init
                    ),
            ObjectType::BoxedValue => RefObject::BoxedValue(init[0].clone()),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct AbraType {}
