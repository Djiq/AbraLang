use std::{
    any,
    collections::HashMap,
    fmt::Display,
    rc::Rc,
    sync::{atomic::AtomicUsize, Mutex},
};

use crate::{
    typedata::{ObjectType, Type},
    value::{StaticValue, Value},
};
use anyhow::{anyhow, Ok};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct Ref {
    towards: Rc<Mutex<RefHeader>>,
}

impl Ref {
    pub fn get_uuid(&self) -> usize {
        let lock = self.towards.lock().unwrap();
        lock.uuid
    }

    pub fn delete(&self) {
        let mut lock = self.towards.lock().unwrap();
        lock.deleted = true;
        lock.ref_object = RefObject::Null;
    }

    pub fn instance_with(towards: Rc<Mutex<RefHeader>>) -> Ref {
        Ref { towards }
    }

    pub fn get_type(&self) -> Type {
        let lock = self.towards.lock().unwrap();
        lock.get_type()
    }

    pub fn get(&self, at: &Value) -> anyhow::Result<Value> {
        let lock = self.towards.lock().unwrap();
        lock.get(at)
    }

    pub fn modify(&self, at: &Value, with: Value) -> anyhow::Result<()> {
        let mut lock = self.towards.lock().unwrap();
        lock.modify(at, with)
    }
}

#[derive(Debug, Clone)]
pub struct RefHeader {
    pub deleted: bool,
    pub uuid: usize,
    pub ref_object: RefObject,
}
impl RefHeader {
    pub fn instance_with_initializer(typ: ObjectType, args: Vec<Value>) -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        RefHeader {
            deleted: false,
            uuid: COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            ref_object: typ.instance_self(args),
        }
    }

    pub fn get_type(&self) -> Type {
        match &self.ref_object {
            RefObject::Map(t1, t2, map) => {
                        Type::Object(ObjectType::Map(Box::new(t1.clone()), Box::new(t2.clone())))
                    }
            RefObject::Array(typ, val) => Type::Object(ObjectType::Array(Box::new(typ.clone()))),
            RefObject::Null => Type::Object(ObjectType::Null),
            RefObject::BoxedValue(value) => Type::Object(ObjectType::BoxedValue),
        }
    }

    pub fn get(&self, at: &Value) -> anyhow::Result<Value> {
        match &self.ref_object {
            RefObject::Map(t1, t2, map) => Ok(map[&at.get_string_representation()].clone()),
            RefObject::Null => Err(anyhow!("Cannot dereference null")),
            RefObject::Array(typ, arr) => {
                        let index = at.expect_int()?;
                        Ok(arr[index as usize].clone())
                    }
            RefObject::BoxedValue(value) => Ok(value.clone()),
        }
    }

    pub fn modify(&mut self, at: &Value, with: Value) -> anyhow::Result<()> {
        match &mut self.ref_object {
            RefObject::Map(t1, t2, map) => {
                let txt = at.get_string_representation();
                if map.contains_key(&txt) {
                    map.remove(&txt);
                }
                map.insert(txt, with);
                Ok(())
            }
            RefObject::Null => Err(anyhow!("Cannot dereference null")),
            RefObject::Array(typ, arr) => {
                let index = at.expect_int()?;
                arr[index as usize] = with;
                Ok(())
            }
            RefObject::BoxedValue(value) => {
                *value = with.clone();
                Ok(())
            },
        }
    }
}

#[derive(Debug, Clone)]
pub enum RefObject {
    Null,
    BoxedValue(Value),
    Array(Type, Vec<Value>),
    Map(Type, Type, HashMap<String, Value>),
}

impl Display for RefObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RefObject::Map(t1, t2, map) => {
                        write!(f, "<>")
                    }
            RefObject::Null => write!(f, "null"),
            RefObject::Array(typ, arr) => {
                        let s = arr
                            .iter()
                            .map(|v| v.to_string())
                            .fold(String::new(), |acc, v| format!("{},{}", acc, v));
                        write!(f, "[{}]", s)
                    }
RefObject::BoxedValue(value) => todo!(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, PartialOrd)]
pub struct ObjectInitializer {
    pub typ: ObjectType,
}

impl ObjectInitializer {
    
}
