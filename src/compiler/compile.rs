use std::{any, collections::HashMap};

use anyhow::*;
use serde::{Deserialize, Serialize};

use crate::{
    compiler::{
        typecheck::{AbraTypeDefinition, FunctionSignature, Type, TypeChecker, TypeCheckerMessage},
        ByteCode,
    },
    frontend::{
        ast::{BinOpCode, Expression, Item, Statement},
        tokenizer::TokenLiteral,
    },
};

#[derive(Serialize, Deserialize, Clone)]
pub struct Code {
    pub bytecode: Vec<ByteCode>,
    pub labels: Vec<(String, usize)>,
}

impl Code {
    pub fn string_representation(&self) -> String {
        let mut ret = String::new();
        for byte in self.bytecode.iter().enumerate() {
            for label in self.labels.iter() {
                if label.1 == byte.0 {
                    ret.push_str(&format!("{} | {}:\n", label.1, label.0));
                }
            }
            ret.push_str(&format!(
                "{} | {}:\n",
                byte.0,
                serde_json::to_string(&byte.1).unwrap()
            ));
        }
        ret
    }
}

impl From<Compiler> for Code {
    fn from(value: Compiler) -> Self {
        Code {
            bytecode: value.get_code(),
            labels: value.get_labels(),
        }
    }
}

type CompFuncSig = (Option<usize>, FunctionSignature);
#[derive(Serialize, Deserialize, Clone)]
pub enum Symbol {
    Class(AbraTypeDefinition),
    Function(CompFuncSig),
}

pub struct Compiler {
    bytecode: Vec<ByteCode>,
    labels: Vec<(String, usize)>,
    label_iter: usize,
    symbol_table: HashMap<String, Symbol>,
}

impl Compiler {
    pub fn new() -> Self {
        Compiler {
            bytecode: Vec::new(),
            labels: Vec::new(),
            label_iter: 0,
            symbol_table: HashMap::new(),
        }
    }
    pub fn compilation_pipepline(&mut self, ast: Vec<Item>) -> Result<(), anyhow::Error> {
        self.compile_from_ast(&ast);
        let mut t = TypeChecker::new(&ast);
        t.check();
        for msg in t.messages.iter() {
            println!("{}", msg);
        }
        if t.messages
            .iter()
            .filter(|f| matches!(f, TypeCheckerMessage::Error(_)))
            .count()
            > 0
        {
            println!("Compilation Failed!");
            return Err(anyhow!("Compilation Failed!"));
        }
        let (a, b) = t.export();
        let c = a.iter().map(|a| (a.0.clone(), Symbol::Class(a.1.clone())));
        let d = b
            .iter()
            .map(|b| (b.0.clone(), Symbol::Function((None, b.1.clone()))));
        c.chain(d).for_each(|(k, v)| {
            self.symbol_table.insert(k, v);
        });
        Ok(())
    }

    pub fn compile_from_ast(&mut self, ast: &Vec<Item>) {
        //COMPILATION
        self.labels.push(("_start".into(), 0));
        self.bytecode.push(ByteCode::CALL("main".into(), 0));
        self.bytecode.push(ByteCode::EXIT);

        for item in ast.clone() {
            match item {
                Item::Function(func) => {
                    let mut vec = Vec::new();
                    self.labels.push((func.name, self.bytecode.len()));
                    for arg in func.params.iter().rev() {
                        self.bytecode
                            .push(ByteCode::DEFVAR(arg.name.clone().into(), arg.ty.clone()));
                    }
                    self.compile_body(&func.body, Some(&mut vec));
                }
                Item::Class(class) => {
                    class.functions.iter().for_each(|f| {
                        self.labels.push((
                            format!("{}::{}", class.name.clone(), f.name.clone()),
                            self.bytecode.len(),
                        ));
                        let mut vec = Vec::new();
                        for arg in f.params.iter().rev() {
                            self.bytecode
                                .push(ByteCode::DEFVAR(arg.name.clone().into(), arg.ty.clone()));
                        }
                        self.compile_body(&f.body, Some(&mut vec));
                    });
                }
            }
        }
    }

    pub fn get_code(&self) -> Vec<ByteCode> {
        self.bytecode.clone()
    }

    pub fn get_labels(&self) -> Vec<(String, usize)> {
        self.labels.clone()
    }

    pub fn get_symbols(&self) -> HashMap<String, Symbol> {
        self.symbol_table.clone()
    }
    ///>

    pub fn string_representation(&self) -> String {
        let mut ret = String::new();
        for byte in self.bytecode.iter().enumerate() {
            for label in self.labels.iter() {
                if label.1 == byte.0 {
                    ret.push_str(&format!("{} | {}:\n", label.1, label.0));
                }
            }
            ret.push_str(&format!(
                "{} | {}:\n",
                byte.0,
                serde_json::to_string(&byte.1).unwrap()
            ));
        }
        ret
    }

    fn get_next_label(&mut self) -> String {
        let ret = format!("_{}", &self.label_iter);
        self.label_iter += 1;
        ret
    }

    fn compile_body(
        &mut self,
        stmts: &Vec<Statement>,
        additional_variables_to_drop_on_scope_end: Option<&mut Vec<String>>,
    ) {
        let drop_vars = additional_variables_to_drop_on_scope_end.is_none();
        let mut vars = Vec::new();
        let vars_to_drop = additional_variables_to_drop_on_scope_end.unwrap_or(&mut vars);
        for stmt in stmts {
            let mut ret: Vec<String> = Vec::new();
            self.compile_statement(stmt, &mut ret);
            vars_to_drop.extend(ret);
        }
        if drop_vars {
            for var_to_drop in vars_to_drop {
                self.bytecode.push(ByteCode::DROPVAR(var_to_drop.clone()));
            }
        }
    }

    fn compile_statement(&mut self, stmt: &Statement, out: &mut Vec<String>) {
        match stmt {
            Statement::Declare(name, typedata, expr) => {
                self.compile_expression(expr);
                self.bytecode
                    .push(ByteCode::DEFVAR(name.clone(), typedata.to_owned()));
                out.push(name.clone());
            }
            Statement::If(expr, block, els) => {
                self.compile_expression(expr);
                self.bytecode.push(ByteCode::NEGATE);
                let lbl = self.get_next_label();
                self.bytecode.push(ByteCode::JITL(lbl.clone()));
                self.compile_body(block, None);
                if els.is_none() {
                    self.labels.push((lbl, self.bytecode.len()));
                } else {
                    self.labels.push((lbl, self.bytecode.len() + 1));
                    let lbl2 = self.get_next_label();
                    self.bytecode.push(ByteCode::JMPTO(lbl2.clone()));
                    self.compile_body(els.as_ref().unwrap(), None);
                    self.labels.push((lbl2, self.bytecode.len()));
                }
            }
            Statement::For(stmt, expr, stmt2, body) => {
                let mut vars = Vec::new();
                self.compile_statement(&stmt, &mut vars);
                let idx = self.bytecode.len();
                self.compile_expression(&expr);
                self.bytecode.push(ByteCode::NEGATE);
                let lbl1 = self.get_next_label();
                self.bytecode.push(ByteCode::JITL(lbl1.clone()));
                if body.is_some() {
                    self.compile_body(body.as_ref().unwrap(), Some(&mut vars));
                }
                self.compile_statement(stmt2, out);

                let lbl2 = self.get_next_label();
                self.bytecode.push(ByteCode::JMPTO(lbl2.clone()));
                self.labels.push((lbl1, self.bytecode.len()));
                for var_to_drop in vars {
                    self.bytecode.push(ByteCode::DROPVAR(var_to_drop));
                }
                self.labels.push((lbl2, idx));
            }
            Statement::Return(op_expr) => {
                if op_expr.is_some() {
                    self.compile_expression(op_expr.as_ref().unwrap());
                    self.bytecode.push(ByteCode::RET(true));
                } else {
                    self.bytecode.push(ByteCode::RET(false));
                }
            }
            Statement::Set(on,variable, expr) => {
                self.compile_expression(expr);
                self.bytecode.push(ByteCode::SAVEVARLOCAL(variable.clone()));
            }
            Statement::Expression(expr) => {
                self.compile_expression(expr);
            }
            Statement::Print(expr) => {
                self.compile_expression(expr);
                //self.bytecode.push(ByteCode::SHOW);
            }
            Statement::Null => {}
        }
    }

    fn compile_expression(&mut self, expr: &Expression) {
        match expr {
            Expression::Get(literal, expr) => {
                self.compile_expression(&expr);

                self.bytecode.push(ByteCode::GETVARLOCAL(literal.clone()));
                self.bytecode.push(ByteCode::GETFROMREF);
            }
            Expression::Literal(literal) => match literal {
                TokenLiteral::Identifier(ident) => {
                    self.bytecode.push(ByteCode::GETVARLOCAL(ident.clone()));
                }
                TokenLiteral::Value(v) => self.bytecode.push(ByteCode::PUSH(v.clone())),
            },
            Expression::Binary(op, lhs, rhs) => {
                self.compile_expression(rhs);
                self.compile_expression(lhs);

                match op {
                    BinOpCode::ADD => self.bytecode.push(ByteCode::ADD),
                    BinOpCode::SUB => self.bytecode.push(ByteCode::SUB),
                    BinOpCode::DIV => self.bytecode.push(ByteCode::DIV),
                    BinOpCode::MULT => self.bytecode.push(ByteCode::MULT),
                    BinOpCode::EQ => self.bytecode.push(ByteCode::EQUALS),
                    BinOpCode::GE => self.bytecode.push(ByteCode::EQGREAT),
                    BinOpCode::LE => self.bytecode.push(ByteCode::EQLESS),
                    BinOpCode::LT => self.bytecode.push(ByteCode::LESSER),
                    BinOpCode::GT => self.bytecode.push(ByteCode::GREATER),
                    _ => {}
                }
            }
            Expression::Call(func, args) => {
                for arg in args {
                    self.compile_expression(arg);
                }
                self.bytecode
                    .push(ByteCode::CALL(func.clone(), args.len() as u64));
            }
            Expression::Unary(op, expr) => {
                self.compile_expression(expr);
                match op {
                    crate::frontend::ast::UnaryOpCode::NEG => self.bytecode.push(ByteCode::NEGATE),
                    crate::frontend::ast::UnaryOpCode::NOT => self.bytecode.push(ByteCode::NOT),
                }
            }
            Expression::Grouping(group) => {
                self.compile_expression(&group);
            }
            Expression::Instance(_t, _expressionss) => {}
        }
    }
}
