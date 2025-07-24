use crate::{
    compiler::typecheck::{
        Composite, FunctionSignature, Primitives, Type, FLOAT_TYPE, INTEGER_TYPE, STRING_TYPE,
    },
    runtime::{value::Value, vm::ByteCodeMachine},
};
use anyhow::*;
use std::{collections::HashMap, rc::Rc};

pub type InbuiltFuncBody = Rc<dyn Fn(&mut ByteCodeMachine, u64) -> anyhow::Result<()>>;
pub type CompleteInbuiltFuncBody = (FunctionSignature, InbuiltFuncBody);
pub type InbuiltFuncMap = HashMap<String, CompleteInbuiltFuncBody>;

struct FuncStore(InbuiltFuncMap);

impl FuncStore {
    fn new() -> Self {
        Self(HashMap::new())
    }

    fn func_gen<T: Into<String>>(
        mut self,
        name: T,
        params: Vec<Type>,
        return_type: Type,
        functor: fn(&mut ByteCodeMachine, u64) -> anyhow::Result<()>,
    ) -> FuncStore {
        let name = name.into();
        let f = FunctionSignature::new(name.clone(), params, return_type);
        self.0.insert(name, (f, Rc::new(functor)));
        self
    }

    fn finalize(self) -> InbuiltFuncMap {
        self.0
    }
}

pub fn generate_inbuilt_function_hashmap() -> InbuiltFuncMap {
    FuncStore::new()
        .func_gen("print", vec![Type::Null], Type::Null, |state, argc| {
            if argc != 1 {
                return Err(anyhow!("Wrong amount of of arguments for print!"));
            }
            let arg0 = state.pop_from_stack()?;
            print!("{}", arg0);
            Ok(())
        })
        .func_gen(
            "sqrt",
            vec![Type::or(FLOAT_TYPE, INTEGER_TYPE)],
            FLOAT_TYPE,
            |state, argc| {
                if argc != 1 {
                    return Err(anyhow!("Wrong amount of of arguments for print!"));
                }
                let arg0 = state.pop_from_stack()?;
                match arg0 {
                    Value::Null => bail!("Wrong type of argument provided: Null"),
                    Value::Integer(i) => {
                        state.push_to_stack(&Value::Integer(f64::sqrt(i as f64) as i64))?;
                    }
                    Value::Float(f) => {
                        state.push_to_stack(&f.sqrt().into())?;
                    }
                    Value::Char(_) => bail!("Wrong type of argument provided: Char"),
                    Value::Bool(_) => bail!("Wrong type of argument provided: Bool"),
                    Value::String(_) => bail!("Wrong type of argument provided: String"),
                    Value::Ref(_) => bail!("Wrong type of argument provided: Ref"),
                }
                Ok(())
            },
        )
        .func_gen(
            "exp",
            vec![Type::or(FLOAT_TYPE, INTEGER_TYPE)],
            FLOAT_TYPE,
            |state, argc| {
                if argc != 1 {
                    return Err(anyhow!("Wrong amount of of arguments for print!"));
                }
                let arg0 = state.pop_from_stack()?;
                match arg0 {
                    Value::Null => bail!("Wrong type of argument provided: Null"),
                    Value::Integer(i) => {
                        state.push_to_stack(&Value::Integer(f64::exp(i as f64) as i64))?;
                    }
                    Value::Float(f) => {
                        state.push_to_stack(&f.exp().into())?;
                    }
                    Value::Char(_) => bail!("Wrong type of argument provided: Char"),
                    Value::Bool(_) => bail!("Wrong type of argument provided: Bool"),
                    Value::String(_) => bail!("Wrong type of argument provided: String"),
                    Value::Ref(_) => bail!("Wrong type of argument provided: Ref"),
                }
                Ok(())
            },
        )
        .func_gen("input", vec![], STRING_TYPE, |_state, argc| {
            if argc != 0 {
                return Err(anyhow!("Wrong amount of of arguments for print!"));
            }
            Ok(())
        })
        .finalize()
}
