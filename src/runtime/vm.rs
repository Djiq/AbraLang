use crate::{
    compiler::{
        typecheck::{AbraTypeDefinition, Type},
        ByteCode, Code,
    },
    runtime::inbuilt::generate_inbuilt_function_hashmap,
};
use anyhow::*;
use std::{collections::HashMap, io::BufRead, rc::Rc, sync::Mutex};

use super::{
    object::{Ref, RefHeader},
    // types::{ObjectType, Type}, // Old type system import
    value::Value,
};

/*
    R0 - LONG-LASTING REGISTER 0
    R1 - LONG-LASTING REGISTER 1
    R2 - LONG-LASTING REGISTER 2
    R3 - LONG-LASTING REGISTER 3
    R4 - LONG-LASTING REGISTER 4
    R5 - LONG-LASTING REGISTER 5
    R6 - LONG-LASTING REGISTER 6
    R7 - LONG-LASTING REGISTER 7
    R8 - LONG-LASTING REGISTER 8
    R9 - LONG-LASTING REGISTER 9
    RSI - STACK INDEX
    RBI - BYTECODE INDEX
    RSP - STACK FRAME POINTER
*/
pub struct ByteCodeMachine {
    bytecode: Vec<ByteCode>,
    labels: HashMap<String, usize>,

    registers: [Value; 16],
    global_variables: HashMap<String, Value>,
    stack_frames: Vec<StackFrame>,
    stack: [Value; 1028],

    debug_mode: bool,
    debug_run: bool,
    debug_show_stack: bool,
    debug_show_bytecode: bool,
    debug_breakpoints: Vec<usize>,
    // This should now use the new AbraTypeDefinition from compiler::typecheck
    abra_types: Vec<AbraTypeDefinition>,
    inbuilt_functions: HashMap<
        String,
        (
            crate::compiler::typecheck::FunctionSignature,
            Rc<dyn Fn(&mut ByteCodeMachine, u64) -> anyhow::Result<()>>,
        ),
    >,
}

struct StackFrame {
    name: Option<String>,
    local_variables: HashMap<String, Value>,
    object: Option<Ref>,
    bytecode_return_index: i64,
    stack_return_index: i64,
}

impl StackFrame {
    fn new<T: Into<String>>(
        bytecode_ret_index: i64,
        stack_ret_index: i64,
        name: Option<T>,
    ) -> Self {
        StackFrame {
            name: name.map_or(None, |s| Some(s.into())),
            local_variables: HashMap::new(),
            object: None,
            bytecode_return_index: bytecode_ret_index,
            stack_return_index: stack_ret_index,
        }
    }
}

impl ByteCodeMachine {
    pub fn new(code: Code, debug_mode: bool) -> Self {
        let mut slf = ByteCodeMachine {
            bytecode: code.bytecode,
            registers: [const { Value::Null }; 16],
            labels: code
                .labels
                .into_iter()
                .map(|(k, v)| (k.into(), v))
                .collect(),
            global_variables: HashMap::new(),
            stack_frames: Vec::new(),
            stack: [const { Value::Null }; 1028],
            debug_mode,
            debug_run: false,
            debug_show_bytecode: false,
            debug_show_stack: false,
            debug_breakpoints: Vec::new(),
            abra_types: Vec::new(),
            inbuilt_functions: generate_inbuilt_function_hashmap(),
        };
        let start_index = slf.labels["_start"];
        slf.registers[11] = Value::Integer(start_index as i64);
        slf.registers[10] = Value::Integer(0);
        if debug_mode {
            println!("DEBUG MODE <Q/q - quit> <R/r - run> <N/n - next> <B/b - set breakpoint> <S/s - shows first 10 values on stack> <C/c - shows bytecode>");
        }
        slf
    }

    fn instance(&mut self, typ: Type, values: Vec<Value>) -> Ref {
        Ref::instance_with(Rc::new(Mutex::new(RefHeader::instance_with_initializer(
            typ,
            values,
            &self.abra_types,
        ))))
    }

    fn delete(&mut self, reference: Value) -> anyhow::Result<()> {
        let rf = reference.expect_ref()?;
        rf.delete();
        Ok(())
    }

    fn debug_mode(&mut self) -> bool {
        use std::io::{stdin, Read};
        let mut stdin_handle = stdin().lock();
        let mut byte = [0_u8];
        if self.debug_show_bytecode {
            println!("Bytecode:");
            let index = self.registers[11].expect_int().unwrap() as usize;
            let (low_range, high_range) = (
                0.max(index as i64 - 5) as usize,
                self.bytecode.len().min(index + 5),
            );
            for i in low_range..high_range {
                print!(
                    "{} | {}",
                    i,
                    serde_json::to_string(&self.bytecode[i]).unwrap()
                );
                if i == index {
                    println!(" << CURRENT");
                } else {
                    println!("");
                }
            }
        }
        if self.debug_show_stack {
            println!("Stack:");
            let stack_index = self.registers[10].expect_int().unwrap();
            let mut i = stack_index;
            while i >= 0 && i + 10 >= stack_index {
                if i == stack_index {
                    println!("{} | {} << HEAD", i, &self.stack[i as usize]);
                } else {
                    println!("{} | {}", i, &self.stack[i as usize]);
                }

                i -= 1;
            }
        }
        loop {
            if self.debug_run {
                let index = self.registers[11].expect_int().unwrap() as usize;
                if self.debug_breakpoints.contains(&index) {
                    self.debug_run = false;
                    continue;
                }
                break;
            }
            stdin_handle.read_exact(&mut byte).unwrap();
            let character: char = byte[0] as char;
            //println!("{}",character);
            match character {
                'c' | 'C' => {
                    self.debug_show_bytecode = !self.debug_show_bytecode;
                }
                's' | 'S' => {
                    self.debug_show_stack = !self.debug_show_stack;
                }
                'b' | 'B' => {
                    let mut string = String::new();
                    stdin_handle.read_line(&mut string).unwrap();
                    let stop_on: usize = string.trim().parse().unwrap();
                    //println!("{}",string);
                    self.debug_breakpoints.push(stop_on);
                    continue;
                }
                'r' | 'R' => {
                    self.debug_run = true;
                    break;
                }
                'n' | 'N' => break,
                'q' | 'Q' => return true,
                _ => {}
            }
        }
        false
    }

    pub fn run(&mut self) -> usize {
        loop {
            if self.debug_mode {
                let q = self.debug_mode();
                if q {
                    return 1;
                }
            }
            match self.next() {
                Result::Ok(true) => {
                    self.registers[11] = self.registers[11].clone() + Value::Integer(1);
                    continue;
                }
                Result::Ok(false) => {
                    println!("Program exited successfully.");
                    return self.pop_from_stack().unwrap().expect_int().unwrap() as usize;
                }
                Err(e) => {
                    println!("An error occureed!\n {}", e);
                    for stack in &self.stack_frames {
                        println!(
                            "From <{}>",
                            stack.name.as_ref().unwrap_or(&"unknown".into())
                        );
                    }
                    return 1;
                }
            }
        }
    }

    pub fn pop_from_stack(&mut self) -> anyhow::Result<Value> {
        let stack_index = self.registers[10].expect_int()? as usize;
        let ret = Ok(self.stack[stack_index - 1].clone());
        self.registers[10] = self.registers[10].clone() - Value::Integer(1);
        //println!("{}",self.registers[10]);
        ret
    }

    pub fn push_to_stack(&mut self, value: &Value) -> anyhow::Result<()> {
        let stack_index = self.registers[10].expect_int()? as usize;
        self.stack[stack_index] = value.clone();
        self.registers[10] = self.registers[10].clone() + Value::Integer(1);
        //println!("{}",self.registers[10]);
        Ok(())
    }

    fn unwind_stack(&mut self) -> anyhow::Result<()> {
        let stack_frame = self.stack_frames.pop().ok_or(anyhow!(
            "Attempted to access stack frames while none are allocated!"
        ))?;
        self.registers[11] = Value::Integer(stack_frame.bytecode_return_index);
        let current_stack_index = self.registers[10].expect_int()?;
        for x in stack_frame.stack_return_index..current_stack_index {
            self.stack[x as usize] = Value::Null;
        }
        self.registers[10] = Value::Integer(stack_frame.stack_return_index);
        Ok(())
    }

    fn clone_value(&mut self, val: &Value) -> Value {
        val.clone()
    }

    fn next(&mut self) -> anyhow::Result<bool> {
        let index = self.registers[11].expect_int()? as usize;
        let code = self.bytecode[index].clone();
        match code {
            ByteCode::PUSH(v) => {
                self.push_to_stack(&v.into())?;
                Ok(true)
            }
            ByteCode::POP => {
                self.pop_from_stack()?;
                Ok(true)
            }
            ByteCode::ADD => {
                let a = self.pop_from_stack()?;
                let b = self.pop_from_stack()?;
                self.push_to_stack(&(a + b))?;
                Ok(true)
            }
            ByteCode::SUB => {
                let b = self.pop_from_stack()?;
                let a = self.pop_from_stack()?;
                self.push_to_stack(&(a - b))?;
                Ok(true)
            }
            ByteCode::MULT => {
                let a = self.pop_from_stack()?;
                let b = self.pop_from_stack()?;
                self.push_to_stack(&(a * b))?;
                Ok(true)
            }
            ByteCode::DIV => {
                let a = self.pop_from_stack()?;
                let b = self.pop_from_stack()?;
                self.push_to_stack(&(a / b))?;
                Ok(true)
            }
            ByteCode::JMPTO(label) => {
                let new_stack_index = self.labels[&label] as i64 - 1;
                self.registers[11] = Value::Integer(new_stack_index);
                Ok(true)
            }
            ByteCode::JMPABS(indx) => {
                self.registers[11] = Value::Integer(indx - 1);
                Ok(true)
            }
            ByteCode::JMPREL(offset) => {
                self.registers[11] = Value::Integer(index as i64 + offset - 1);
                Ok(true)
            }
            ByteCode::JITA(indx) => {
                let boolean = self.pop_from_stack()?.expect_bool()?;
                if boolean {
                    self.registers[11] = Value::Integer(indx - 1);
                }
                Ok(true)
            }
            ByteCode::JITL(label) => {
                let boolean = self.pop_from_stack()?.expect_bool()?;
                if boolean {
                    let new_stack_index = self.labels[&label] as i64 - 1;
                    self.registers[11] = Value::Integer(new_stack_index);
                }
                Ok(true)
            }
            ByteCode::JITR(offset) => {
                let boolean = self.pop_from_stack()?.expect_bool()?;
                if boolean {
                    self.registers[11] = Value::Integer(index as i64 + offset - 1);
                }
                Ok(true)
            }
            ByteCode::AND => {
                let a = self.pop_from_stack()?;
                let b = self.pop_from_stack()?;
                self.push_to_stack(&Value::Bool(a.cast_to_bool()? && b.cast_to_bool()?))?;
                Ok(true)
            }
            ByteCode::OR => {
                let a = self.pop_from_stack()?;
                let b = self.pop_from_stack()?;
                self.push_to_stack(&Value::Bool(a.cast_to_bool()? || b.cast_to_bool()?))?;
                Ok(true)
            }
            ByteCode::XOR => {
                let a = self.pop_from_stack()?;
                let b = self.pop_from_stack()?;
                self.push_to_stack(&Value::Bool(a.cast_to_bool()? ^ b.cast_to_bool()?))?;
                Ok(true)
            }
            ByteCode::NEGATE => {
                let a = self.pop_from_stack()?;
                self.push_to_stack(&Value::Bool(!a.cast_to_bool()?))?;
                Ok(true)
            }
            ByteCode::EQUALS => {
                let a = self.pop_from_stack()?;
                let b = self.pop_from_stack()?;
                self.push_to_stack(&Value::Bool(a == b))?;
                Ok(true)
            }
            ByteCode::EQGREAT => {
                let a = self.pop_from_stack()?;
                let b = self.pop_from_stack()?;
                self.push_to_stack(&Value::Bool(a >= b))?;
                Ok(true)
            }
            ByteCode::EQLESS => {
                let a = self.pop_from_stack()?;
                let b = self.pop_from_stack()?;
                self.push_to_stack(&Value::Bool(a <= b))?;
                Ok(true)
            }
            ByteCode::GREATER => {
                let a = self.pop_from_stack()?;
                let b = self.pop_from_stack()?;
                self.push_to_stack(&Value::Bool(a > b))?;
                Ok(true)
            }
            ByteCode::LESSER => {
                let a = self.pop_from_stack()?;
                let b = self.pop_from_stack()?;
                self.push_to_stack(&Value::Bool(a < b))?;
                Ok(true)
            }
            ByteCode::DUP => {
                let a = self.pop_from_stack()?;
                self.push_to_stack(&a)?;
                self.push_to_stack(&a)?;
                Ok(true)
            }
            ByteCode::SAVEVARGLOBAL(name) => {
                let a = self.pop_from_stack()?;
                if self.global_variables.contains_key(&name) {
                    *self
                        .global_variables
                        .get_mut(&name)
                        .ok_or(anyhow!("Bad variable name while saving a variable!"))? = a;
                } else {
                    self.global_variables.insert(name.to_string(), a);
                }
                Ok(true)
            }
            ByteCode::GETVARGLOBAL(name) => {
                let value = self
                    .global_variables
                    .get(&name)
                    .ok_or(anyhow!("Attempted to access an undefined variable!"))?
                    .clone();
                let cloned_val = self.clone_value(&value);
                self.push_to_stack(&cloned_val)?;

                Ok(true)
            }
            ByteCode::SAVEVARLOCAL(name) => {
                if self.stack_frames.is_empty() {
                    return Err(anyhow!(
                        "Attempted to access stack frames while none are allocated!"
                    ));
                }
                let a = self.pop_from_stack()?;
                let b = self
                    .stack_frames
                    .last()
                    .ok_or(anyhow!(
                        "Attempted to access stack frames while none are allocated!"
                    ))?
                    .local_variables
                    .contains_key(&name);
                if b {
                    *self
                        .stack_frames
                        .last_mut()
                        .ok_or(anyhow!(
                            "Attempted to access stack frames while none are allocated!"
                        ))?
                        .local_variables
                        .get_mut(&name)
                        .ok_or(anyhow!(
                            "Attempted to access stack frame variables while none are allocated!"
                        ))? = a;
                } else {
                    self.stack_frames
                        .last_mut()
                        .ok_or(anyhow!(
                            "Attempted to access stack frames while none are allocated!"
                        ))?
                        .local_variables
                        .insert(name.to_string(), a);
                }
                Ok(true)
            }
            ByteCode::GETVARLOCAL(name) => {
                if self.stack_frames.is_empty() {
                    return Err(anyhow!(
                        "Attempted to access stack frames while none are allocated!"
                    ));
                }

                let value = self
                    .stack_frames
                    .last()
                    .ok_or(anyhow!(
                        "Attempted to access stack frames while none are allocated!"
                    ))?
                    .local_variables
                    .get(&name)
                    .ok_or(anyhow!("Attempted to access an undefined variable!"))?
                    .clone();
                let cloned_val = self.clone_value(&value);
                self.push_to_stack(&cloned_val)?;

                Ok(true)
            }
            ByteCode::CALL(func, argc) => {
                if self.inbuilt_functions.contains_key(&func) {
                    self.inbuilt_functions.get(&func).unwrap().1.clone()(self, argc)?;
                    return Ok(true);
                }
                let mut argv = Vec::new();
                for _ in 0..argc {
                    argv.push(self.pop_from_stack()?);
                }
                self.stack_frames.push(StackFrame::new(
                    index as i64,
                    self.registers[10].expect_int()?,
                    Some(&func),
                ));
                let new_bc_index = self.labels[&func] as i64 - 1;
                self.registers[11] = Value::Integer(new_bc_index);

                Ok(true)
            }
            ByteCode::RET(return_value) => {
                let mut returning_value: Option<Value> = None;
                if return_value {
                    returning_value = Some(self.pop_from_stack()?);
                }
                self.unwind_stack()?;
                if return_value {
                    self.push_to_stack(returning_value.as_ref().unwrap())?;
                }
                Ok(true)
            }
            ByteCode::EXIT => Ok(false),
            ByteCode::INSTANCE(typ, argc) => {
                let mut acc = Vec::new();
                for _ in 0..argc {
                    acc.push(self.pop_from_stack()?);
                }
                let rf = self.instance(typ, acc);
                self.push_to_stack(&Value::Ref(rf))?;
                Ok(true)
            }
            ByteCode::GETFROMREF => {
                let value = {
                    let rf = self.pop_from_stack()?.expect_ref()?;
                    let offset = self.pop_from_stack()?;
                    rf.get(&offset)?
                };

                self.push_to_stack(&value)?;
                Ok(true)
            }
            ByteCode::SAVETOREF => {
                let value = self.pop_from_stack()?;
                let rf = self.pop_from_stack()?.expect_ref()?;
                let offset = self.pop_from_stack()?;
                rf.modify(&offset, value)?;

                Ok(true)
            }
            ByteCode::DEFVAR(string, _t) => {
                let val = self.pop_from_stack()?;
                self.stack_frames
                    .last_mut()
                    .ok_or(anyhow!(
                        "Attempted to access stack frames while none are allocated!"
                    ))?
                    .local_variables
                    .insert(string, val);
                Ok(true)
            }
            ByteCode::DROPVAR(string) => {
                self.stack_frames
                    .last_mut()
                    .ok_or(anyhow!(
                        "Attempted to access stack frames while none are allocated!"
                    ))?
                    .local_variables
                    .remove_entry(&string);
                Ok(true)
            }
            ByteCode::CAST(typ) => {
                let val = self.pop_from_stack()?;
                self.push_to_stack(&val.cast(typ)?)?;
                Ok(true)
            }
            ByteCode::MOD => {
                let a = self.pop_from_stack()?;
                let b = self.pop_from_stack()?;
                self.push_to_stack(&Value::Integer(a.expect_int()? % b.expect_int()?))?;
                Ok(true)
            }
            ByteCode::NOT => {
                let val = self.pop_from_stack()?;
                self.push_to_stack(&Value::Bool(!val.cast_to_bool()?))?;
                Ok(true)
            }
        }
    }
}
