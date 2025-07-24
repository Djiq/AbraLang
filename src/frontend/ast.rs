use std::fmt::Display;

use crate::{
    compiler::typecheck::Type,
    frontend::tokenizer::{Token, TokenLiteral},
    runtime::value::StaticValue,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Class(Class),
    Function(Function),
}
#[derive(Debug, Clone, PartialEq)]
pub struct Class {
    pub name: String,
    pub variables: Vec<(String, Type, StaticValue)>,
    pub functions: Vec<Function>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Type,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOpCode {
    ADD,
    SUB,
    MULT,
    DIV,
    MOD,
    AND,
    OR,
    XOR,
    LT,
    LE,
    GT,
    GE,
    EQ,
    NE,
}

impl From<Token> for BinOpCode {
    fn from(value: Token) -> Self {
        match value {
            Token::Plus => BinOpCode::ADD,
            Token::Minus => BinOpCode::SUB,
            Token::Star => BinOpCode::MULT,
            Token::Slash => BinOpCode::DIV,
            Token::Lesser => BinOpCode::LT,
            Token::EqualsLesser => BinOpCode::LE,
            Token::Greater => BinOpCode::GT,
            Token::EqualsGreater => BinOpCode::GE,
            Token::EqualsEquals => BinOpCode::EQ,
            Token::BangEq => BinOpCode::NE,
            _ => panic!(),
        }
    }
}

impl<T: Into<String>> From<T> for BinOpCode {
    fn from(value: T) -> Self {
        let s: String = value.into();
        match s.as_str() {
            "+" => BinOpCode::ADD,
            "-" => BinOpCode::SUB,
            "*" => BinOpCode::MULT,
            "/" => BinOpCode::DIV,
            "%" => BinOpCode::MOD,
            "&" => BinOpCode::AND,
            "|" => BinOpCode::OR,
            "^" => BinOpCode::XOR,
            "<" => BinOpCode::LT,
            "<=" => BinOpCode::LE,
            ">" => BinOpCode::GT,
            ">=" => BinOpCode::GE,
            "==" => BinOpCode::EQ,
            "!=" => BinOpCode::NE,
            _ => panic!(),
        }
    }
}

impl Display for BinOpCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinOpCode::ADD => write!(f, "+"),
            BinOpCode::SUB => write!(f, "-"),
            BinOpCode::MULT => write!(f, "*"),
            BinOpCode::DIV => write!(f, "/"),
            BinOpCode::MOD => write!(f, "%"),
            BinOpCode::AND => write!(f, "&"),
            BinOpCode::OR => write!(f, "|"),
            BinOpCode::XOR => write!(f, "^"),
            BinOpCode::LT => write!(f, "<"),
            BinOpCode::LE => write!(f, "<="),
            BinOpCode::GT => write!(f, ">"),
            BinOpCode::GE => write!(f, ">="),
            BinOpCode::EQ => write!(f, "=="),
            BinOpCode::NE => write!(f, "!="),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOpCode {
    NEG,
    NOT,
}

impl Display for UnaryOpCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnaryOpCode::NEG => write!(f, "-"),
            UnaryOpCode::NOT => write!(f, "!"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Literal(TokenLiteral),
    Unary(UnaryOpCode, Box<Expression>),
    Binary(BinOpCode, Box<Expression>, Box<Expression>),
    Grouping(Box<Expression>),
    Call(String, Vec<Expression>),
    Get(String, Box<Expression>),
    Instance(Type, Vec<Expression>),
}
//Generate Display trait implementation for Expression enum
impl Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expression::Literal(literal) => write!(f, "{}", literal),
            Expression::Unary(op, expr) => write!(f, "{}{}", op, expr),
            Expression::Binary(op, lhs, rhs) => write!(f, "({} {} {})", lhs, op, rhs),
            Expression::Grouping(expr) => write!(f, "({})", expr),
            Expression::Call(func, args) => {
                write!(f, "{}(", func)?;
                for (i, arg) in args.iter().enumerate() {
                    write!(f, "{}", arg)?;
                    if i < args.len() - 1 {
                        write!(f, ", ")?;
                    }
                }
                write!(f, ")")
            }
            Expression::Get(literal, expr) => write!(f, "{}.{}", expr, literal),
            Expression::Instance(t, expressionss) => {
                write!(f, "new {} {{", t)?;
                for (i, expr) in expressionss.iter().enumerate() {
                    write!(f, "{}", expr)?;
                    if i < expressionss.len() - 1 {
                        write!(f, ", ")?;
                    }
                }
                write!(f, "}}")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Declare(String, Type, Expression),
    Set(Option<Expression>,String, Expression),
    Expression(Expression),
    Print(Expression),
    Return(Option<Expression>),
    If(Expression, Vec<Statement>, Option<Vec<Statement>>),
    For(
        Box<Statement>,
        Expression,
        Box<Statement>,
        Option<Vec<Statement>>,
    ),
    Null,
}

//Generate Display trait impl for Statement
impl Display for Statement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Statement::Declare(name, type_data, expr) => {
                write!(f, "let {}: {} = {}", name, type_data, expr)
            }
            Statement::Set(on,name, expr) => {
                if let Some(on) = on {
                    write!(f, "{}.", on)?;
                }   
                write!(f, "{} = {}", name, expr)
            },
            Statement::Expression(expr) => write!(f, "{}", expr),
            Statement::Print(expr) => write!(f, "print {}", expr),
            Statement::Return(op_expr) => {
                write!(f, "return")?;
                if let Some(expr) = op_expr {
                    write!(f, " {}", expr)?;
                }
                Ok(())
            }
            Statement::If(expr, block, els) => {
                write!(f, "if {} {{\n", expr)?;
                for stmt in block {
                    write!(f, "{}\n", stmt)?;
                }
                write!(f, "}}")?;
                if let Some(else_block) = els {
                    write!(f, " else {{\n")?;
                    for stmt in else_block {
                        write!(f, "{}\n", stmt)?;
                    }
                    write!(f, "}}")?;
                }
                Ok(())
            }
            Statement::For(stmt, expr, stmt2, body) => {
                write!(f, "for {} {} {}", stmt, expr, stmt2)?;
                if let Some(body) = body {
                    write!(f, " {{\n")?;
                    for stmt in body {
                        write!(f, "{}\n", stmt)?;
                    }
                    write!(f, "}}")?;
                }
                Ok(())
            }
            Statement::Null => write!(f, ""),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub ty: Type,
}
