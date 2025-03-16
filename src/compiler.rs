use serde::{Deserialize, Serialize};

use crate::{
    object::ObjectInitializer, tokenizer::TokenLiteral, ByteCode, Expression, Function, Statement,
    StaticValue, Token,
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

pub struct Compiler {
    bytecode: Vec<ByteCode>,
    labels: Vec<(String, usize)>,
    label_iter: usize,
}

impl Compiler {
    pub fn new() -> Self {
        Compiler {
            bytecode: Vec::new(),
            labels: Vec::new(),
            label_iter: 0,
        }
    }

    pub fn compile_from_ast(&mut self, ast: Vec<Function>) {
        self.labels.push(("_start".into(), 0));
        self.bytecode.push(ByteCode::CALL("main".into()));
        self.bytecode.push(ByteCode::EXIT);
        for func in ast {
            let mut vec = Vec::new();
            self.labels.push((func.name, self.bytecode.len()));
            self.compile_body(&func.body, Some(&mut vec));
        }
    }

    pub fn get_code(&self) -> Vec<ByteCode> {
        self.bytecode.clone()
    }

    pub fn get_labels(&self) -> Vec<(String, usize)> {
        self.labels.clone()
    }

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
                self.bytecode.push(ByteCode::DEFVAR(name.clone()));
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
            Statement::Assign(variable, expr) => {
                self.compile_expression(expr);
                self.bytecode.push(ByteCode::SAVEVARLOCAL(variable.clone()));
            }
            Statement::Expression(expr) => {
                self.compile_expression(expr);
            }
            Statement::Print(expr) => {
                self.compile_expression(expr);
                self.bytecode.push(ByteCode::SHOW);
            }
            Statement::Null => {}
        }
    }

    fn compile_expression(&mut self, expr: &Expression) {
        match expr {
            Expression::Access(literal, expr) => {
                self.compile_expression(&expr);
                if let TokenLiteral::Identifier(ident) = literal {
                    self.bytecode.push(ByteCode::GETVARLOCAL(ident.clone()));
                    self.bytecode.push(ByteCode::GETFROMREF);
                }
            }
            Expression::List(typ, literals) => {
                let mut values = Vec::new();
                for literal in literals {
                    match literal {
                        crate::TokenLiteral::Integer(i) => values.push(StaticValue::Integer(*i)),
                        crate::TokenLiteral::Bool(i) => values.push(StaticValue::Bool(*i)),
                        crate::TokenLiteral::Float(i) => values.push(StaticValue::Float(*i)),
                        crate::TokenLiteral::String(s) => {
                            values.push(StaticValue::String(s.clone()))
                        }
                        crate::TokenLiteral::Char(c) => values.push(StaticValue::Char(*c)),
                        _ => {}
                    }
                }
                self.bytecode.push(ByteCode::INSTANCE(ObjectInitializer {
                    typ: crate::typedata::ObjectType::Array(Box::new(typ.clone())),
                    init: literals.iter().map(|lit| lit.to_static_value()).collect(),
                }));
            }
            Expression::Literal(literal) => match literal {
                crate::TokenLiteral::Identifier(ident) => {
                    self.bytecode.push(ByteCode::GETVARLOCAL(ident.clone()));
                }
                crate::TokenLiteral::Integer(i) => {
                    self.bytecode.push(ByteCode::PUSH(StaticValue::Integer(*i)))
                }
                crate::TokenLiteral::Bool(i) => {
                    self.bytecode.push(ByteCode::PUSH(StaticValue::Bool(*i)))
                }
                crate::TokenLiteral::Float(i) => {
                    self.bytecode.push(ByteCode::PUSH(StaticValue::Float(*i)))
                }
                crate::TokenLiteral::String(s) => self
                    .bytecode
                    .push(ByteCode::PUSH(StaticValue::String(s.clone()))),
                crate::TokenLiteral::Char(c) => {
                    self.bytecode.push(ByteCode::PUSH(StaticValue::Char(*c)))
                }
            },
            Expression::Binary(op, lhs, rhs) => {
                self.compile_expression(rhs);
                self.compile_expression(lhs);

                match op {
                    Token::Plus => self.bytecode.push(ByteCode::ADD),
                    Token::Minus => self.bytecode.push(ByteCode::SUB),
                    Token::Slash => self.bytecode.push(ByteCode::DIV),
                    Token::Star => self.bytecode.push(ByteCode::MULT),
                    Token::EqualsEquals => self.bytecode.push(ByteCode::EQUALS),
                    Token::EqualsGreater => self.bytecode.push(ByteCode::EQGREAT),
                    Token::EqualsLesser => self.bytecode.push(ByteCode::EQLESS),
                    Token::Lesser => self.bytecode.push(ByteCode::LESSER),
                    Token::Greater => self.bytecode.push(ByteCode::GREATER),
                    _ => {}
                }
            }
            Expression::Call(func, args) => {
                if let TokenLiteral::Identifier(func_name) = func {
                    self.bytecode.push(ByteCode::CALL(func_name.clone()));
                }
            }
            Expression::Unary(op, expr) => {}
            Expression::Grouping(group) => {
                self.compile_expression(&group);
            }
        }
    }
}
