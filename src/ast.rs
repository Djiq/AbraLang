use std::fmt::Display;

use crate::{token::Token, token::TokenLiteral, typedata::Type};


pub enum Item{
    Function(Function)
}

pub struct Function {
    pub name: String,
    pub return_type: Token,
    pub body: Vec<Statement>,
}

#[derive(Clone)]
pub enum BinOpCode{
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
    NE
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
            _ => panic!()
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
            _ => panic!()
        }
    }
}

impl Display for BinOpCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinOpCode::ADD => write!(f,"+"),
            BinOpCode::SUB => write!(f,"-"),
            BinOpCode::MULT => write!(f,"*"),
            BinOpCode::DIV => write!(f,"/"),
            BinOpCode::MOD => write!(f,"%"),
            BinOpCode::AND => write!(f,"&"),
            BinOpCode::OR => write!(f,"|"),
            BinOpCode::XOR => write!(f,"^"),
            BinOpCode::LT => write!(f,"<"),
            BinOpCode::LE => write!(f,"<="),
            BinOpCode::GT => write!(f,">"),
            BinOpCode::GE => write!(f,">="),
            BinOpCode::EQ => write!(f,"=="),
            BinOpCode::NE => write!(f,"!="),
        }
    }
}

#[derive(Clone)]
pub enum UnaryOpCode{
    NEG
}

impl Display for UnaryOpCode{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnaryOpCode::NEG => write!(f,"!"),
        }
    }
}

#[derive(Clone)]
pub enum Expression {
    Literal(TokenLiteral),
    Unary(UnaryOpCode, Box<Expression>),
    Binary(BinOpCode, Box<Expression>, Box<Expression>),
    Grouping(Box<Expression>),
    Call(String, Vec<Expression>),
    Access(String, Box<Expression>),
    Instance(Type,Vec<Expression>)
}

impl Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Expression::Call(func, args) => write!(f, "{}", func),
            Expression::Literal(x) => write!(f, "[{}]", x),
            Expression::Binary(op, l, r) => write!(f, "[{} {} {}]", l, op, r),
            Expression::Unary(op, x) => write!(f, "[{} {}]", op, x),
            Expression::Grouping(x) => write!(f, "[ ( {} ) ]", x),
            _ => write!(f, ""),
        }
    }
}

pub enum Statement {
    Declare(String, Type, Expression),
    Assign(String, Expression),
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
