use std::{
    collections::HashMap,
    fmt::Display,
    hash::{Hash, Hasher},
    rc::Rc,
    sync::{atomic::AtomicUsize, Mutex},
};

use crate::{
    compiler::typecheck::{AbraTypeDefinition, Composite, Primitives, Type},
    runtime::value::Value,
};

use anyhow::{anyhow, Ok, Result};
//use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct Ref {
    towards: Rc<Mutex<RefHeader>>,
}

impl PartialEq for Ref {
    fn eq(&self, other: &Self) -> bool {
        let lock1 = self.get_uuid();
        let lock2 = other.get_uuid();
        lock1 == lock2
    }
}

impl Eq for Ref {}

impl Ref {
    pub fn call_virt<T: Into<String>>(&self, func_name: T, args_vec: Vec<Value>) -> Result<Value> {
        let mut lock = self.towards.lock().unwrap();
        lock.call_virt(func_name, args_vec)
    }

    pub fn get_uuid(&self) -> usize {
        let lock = self.towards.lock().unwrap();
        lock.uuid
    }

    pub fn is_null(&self) -> bool {
        let lock = self.towards.lock().unwrap();
        lock.deleted
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
        lock.set(at, with)
    }
}

impl Display for Ref {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let lock = self.towards.lock().unwrap();
        lock.ref_object.fmt(f)
    }
}

#[derive(Debug, Clone)]
pub struct RefHeader {
    pub deleted: bool,
    pub uuid: usize,
    pub ref_object: RefObject,
}

impl Hash for RefHeader {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.deleted.hash(state);
        self.uuid.hash(state);
        self.ref_object.hash(state);
    }
}

impl RefHeader {
    pub fn instance_with_initializer(
        typ: Type,
        args: Vec<Value>,
        type_tree: &Vec<AbraTypeDefinition>,
    ) -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        RefHeader {
            deleted: false,
            uuid: COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            ref_object: match typ.clone() {
                Type::Primitive(p) => {
                    // For primitives, we typically create a BoxedValue.
                    // The initial value depends on the arguments, usually the first one.
                    let initial_val = args.get(0).cloned().unwrap_or_default();
                    RefObject::BoxedValue(initial_val, typ)
                }
                Type::Composite(composite_box) => match *composite_box {
                    Composite::Array(element_type) => RefObject::Array(element_type, args),
                    Composite::Map(key_type, value_type) => {
                        let mut map = HashMap::new();
                        if args.len() % 2 != 0 {
                            // Or handle error appropriately
                            eprintln!("Warning: Odd number of arguments for map initialization. Ignoring last argument.");
                        }
                        for chunk in args.chunks_exact(2) {
                            map.insert(chunk[0].clone(), chunk[1].clone());
                        }
                        RefObject::Map(key_type, value_type, map)
                    }
                    Composite::HeapValue(value_type) => {
                        let initial_val = args.get(0).cloned().unwrap_or_default();
                        RefObject::BoxedValue(initial_val, value_type)
                    }
                },
                Type::Algebraic(_) => {
                    // Cannot directly instantiate an algebraic type.
                    RefObject::Null // Or handle as an error
                }
                Type::Abra(abra_type_name) => {
                    match type_tree.iter().find(|def| def.name == abra_type_name) {
                        Some(def) => RefObject::Abra(AbraObject::new(def.clone(), args)),
                        None => panic!("Abra type definition not found: {}", abra_type_name), // Or return error
                    }
                }
                Type::Null => RefObject::Null,
            },
        }
    }

    pub fn call_virt<T: Into<String>>(
        &mut self,
        _func_name: T,
        _args_vec: Vec<Value>,
    ) -> Result<Value> {
        match &self.ref_object {
            RefObject::Null => Err(anyhow!("Cannot call a function on a Null Ref")),
            RefObject::BoxedValue(_, _) => Err(anyhow!("Cannot call a function on a Value Ref")),
            RefObject::Array(_, _) => Err(anyhow!("Cannot call a virtual function on a Array Ref")),
            RefObject::Map(_, _, _) => Err(anyhow!("Cannot call a virtual function on a Map Ref")),
            RefObject::Abra(abra_object) => {
                // Placeholder for actual virtual call dispatch
                Err(anyhow!(
                    "Virtual call on AbraObject not yet implemented for function {}",
                    _func_name.into()
                ))
            }
        }
    }

    pub fn get_type(&self) -> Type {
        match &self.ref_object {
            RefObject::Map(t1, t2, _) => {
                Type::Composite(Box::new(Composite::Map(t1.clone(), t2.clone())))
            }
            RefObject::Array(typ, _) => Type::Composite(Box::new(Composite::Array(typ.clone()))),
            RefObject::Null => panic!("Cannot get type of a Null/deleted RefObject"), // Or a specific "Unit" or "Void" type
            RefObject::BoxedValue(_, t) => t.clone(), // The stored type is already the new Type
            RefObject::Abra(abra_object) => Type::Abra(abra_object.abra_type.name.clone()),
        }
    }

    pub fn get(&self, at: &Value) -> anyhow::Result<Value> {
        match &self.ref_object {
            RefObject::Map(_, _, map) => Ok(map[&at].clone()),
            RefObject::Null => Err(anyhow!("Cannot dereference null")),
            RefObject::Array(_, arr) => {
                let index = at.expect_int()?;
                Ok(arr[index as usize].clone())
            }
            RefObject::BoxedValue(value, _) => Ok(value.clone()),
            RefObject::Abra(abra_object) => match at {
                Value::String(var_name) => abra_object.get(var_name),
                _ => Err(anyhow!(
                    "AbraObject access key must be a string variable name"
                )),
            },
        }
    }

    pub fn set(&mut self, at: &Value, with: Value) -> anyhow::Result<()> {
        match &mut self.ref_object {
            RefObject::Map(_, _, map) => {
                if map.contains_key(&at) {
                    map.remove(&at);
                }
                map.insert(at.to_owned(), with);
                Ok(())
            }
            RefObject::Null => Err(anyhow!("Cannot dereference null")),
            RefObject::Array(_, arr) => {
                let index = at.expect_int()?;
                arr[index as usize] = with;
                Ok(())
            }
            RefObject::BoxedValue(value, _) => {
                *value = with.clone();
                Ok(())
            }
            RefObject::Abra(abra_object) => match at {
                Value::String(var_name) => abra_object.set(var_name, with),
                _ => Err(anyhow!(
                    "AbraObject access key must be a string variable name"
                )),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RefObject {
    Null,
    BoxedValue(Value, Type),
    Array(Type, Vec<Value>),
    Map(Type, Type, HashMap<Value, Value>),
    Abra(AbraObject),
}

//write a Hash trait implementation for RefObject
impl Hash for RefObject {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            RefObject::Null => 0.hash(state),
            RefObject::BoxedValue(value, ty) => {
                1.hash(state);
                value.hash(state);
                ty.hash(state);
            }
            RefObject::Array(ty, vec) => {
                2.hash(state);
                ty.hash(state);
                vec.hash(state);
            }
            RefObject::Map(key_ty, value_ty, map) => {
                3.hash(state);
                key_ty.hash(state);
                value_ty.hash(state);
                let mut sorted_pairs: Vec<(&Value, &Value)> = map.iter().collect();
                sorted_pairs.sort_by(|(k1, _), (k2, _)| {
                    k1.partial_cmp(k2).unwrap_or(std::cmp::Ordering::Equal)
                }); // Requires Value to be PartialOrd
                sorted_pairs.hash(state);
            }
            RefObject::Abra(abra_object) => {
                4.hash(state);
                abra_object.abra_type.name.hash(state);
                let mut sorted_vars: Vec<(&String, &Value)> =
                    abra_object.variables.iter().collect();
                sorted_vars.sort_by(|(n1, _), (n2, _)| n1.cmp(n2));
                sorted_vars.hash(state);
            }
        }
    }
}

impl Display for RefObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RefObject::Map(_, _, map) => {
                write!(f, "{{")?;
                let mut first = true;
                for (k, v) in map {
                    if !first {
                        write!(f, ", ")?;
                    }
                    // Use Display impl of Value for keys and values
                    write!(f, "{}: {}", k, v)?;
                    first = false;
                }
                write!(f, "}}")
            }
            RefObject::Null => write!(f, "null"),
            RefObject::Array(_, arr) => {
                write!(f, "[")?;
                let mut first = true;
                for v in arr {
                    if !first {
                        write!(f, ", ")?;
                    }
                    // Use Display impl of Value
                    write!(f, "{}", v)?;
                    first = false;
                }
                write!(f, "")
            }
            RefObject::BoxedValue(value, _) => {
                // Use Display impl of the inner Value
                write!(f, "Box({})", value)
            }
            RefObject::Abra(abra_object) => {
                write!(f, "instance of {}", abra_object.abra_type.name)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AbraObject {
    abra_type: AbraTypeDefinition,
    variables: HashMap<String, Value>,
}

impl AbraObject {
    pub fn new(abra_type: AbraTypeDefinition, args: Vec<Value>) -> AbraObject {
        // TODO: Handle constructor arguments (`args`) properly if/when constructors are implemented.
        // For now, initialize based on type definition defaults.
        let mut variables = HashMap::new();
        for (name, (var_type, is_initialized)) in &abra_type.variables {
            // If we had default values from AST or type system, we'd use them here.
            // For now, just use Value::from(var_type) which gives default for primitives.
            variables.insert(name.clone(), Value::from(var_type.clone()));
        }
        AbraObject {
            abra_type,
            variables,
        }
    }

    pub fn get(&self, var_name: &str) -> anyhow::Result<Value> {
        self.variables.get(var_name).cloned().ok_or_else(|| {
            anyhow!(
                "Variable '{}' not found in instance of {}",
                var_name,
                self.abra_type.name
            )
        })
    }
    pub fn set(&mut self, var_name: &str, value: Value) -> anyhow::Result<()> {
        if self.variables.contains_key(var_name) {
            // TODO: Type check 'value' against self.abra_type.variables[var_name].0
            self.variables.insert(var_name.to_string(), value);
            Ok(())
        } else {
            Err(anyhow!(
                "Variable '{}' not found in instance of {}",
                var_name,
                self.abra_type.name
            ))
        }
    }
}
