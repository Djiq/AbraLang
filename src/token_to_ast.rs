use crate::{ast::{Expression, Function, Statement}, token::{Token, TokenLiteral}, *};

macro_rules! match_token {
    ($self:ident,$pattern:pat $(if $guard:expr)? $(,)?) => {
        if !matches!($self.tokens[$self.index].token(), $pattern) {
            false
        } else {
            //println!("matching {}",$self.tokens[$self.index].token());
            $self.index += 1;
            true
        }
    };
}

macro_rules! to_error {
    ($self:ident,$token:expr) => {
        format!(
            "Error at line: {}, character: {}, hint: {}, token_num: {}\n",
            $token.line + 1,
            $token.character,
            $token.from,
            $self.index
        )
    };
}

macro_rules! to_error_literal {
    ($self:ident,$token:expr) => {
        if let Token::Literal(x) = $token.token() {
            format!(
                "Error at line: {}, character: {}, hint: {}, token_num: {}\n",
                $token.line + 1,
                $token.character,
                x,
                $self.index
            )
        }
    };
}

macro_rules! match_token_or_err {
    ($self:ident,$pattern:pat $(if $guard:expr)? $(,)?) => {
        if !matches!(&$self.tokens[$self.index].token(), $pattern) {
            return Err(anyhow!(
                "Parsing Error: Expected {} found {}\n{}",
                stringify!($pattern),
                &$self.tokens[$self.index].token(),
                to_error!($self, &$self.tokens[$self.index])
            ));
        } else {
            //println!("matching {}",$self.tokens[$self.index].token());
            $self.index = usize::min($self.index + 1, $self.tokens.len() - 1);
        }
    };
}

pub struct Parser {
    index: usize,
    tokens: Vec<TokenData>,
    current_indentation_level: usize,
}

impl Parser {
    pub fn new(tokens: Vec<TokenData>) -> Self {
        Parser {
            index: 0,
            tokens,
            current_indentation_level: 0,
        }
    }

    fn match_type(&mut self) -> Result<Type> {
        self.index += 1;
        match self.tokens[self.index - 1].token() {
            Token::Bool => Ok(Type::Bool),
            Token::Char => Ok(Type::Char),
            Token::Float => Ok(Type::Float),
            Token::Int => Ok(Type::Int),
            Token::LBracket => {
                let ret = Ok(Type::Object(ObjectType::Array(Box::new(
                    self.match_type().with_context(|| {
                        self.index -= 1;
                        format!(
                            "From <match type>, past matched token: {}",
                            self.tokens[self.index - 1].token()
                        )
                    })?,
                ))));
                match_token_or_err!(self, Token::RBracket);
                ret
            }
            _ => {
                self.index -= 1;
                Err(anyhow!(
                    "Expected Type found : {}\n{}",
                    &self.tokens[self.index].token(),
                    to_error!(self, &self.tokens[self.index])
                ))
            }
        }
    }

    pub fn parse(&mut self) -> Result<Vec<Function>> {
        let mut vec = Vec::new();
        while self.index != self.tokens.len()
            && !matches!(self.tokens[self.index].token(), Token::EndOfFile)
        {
            vec.push(self.function().with_context(|| {
                self.index -= 1;
                format!(
                    "From <function>, past matched token: {}",
                    self.tokens[self.index - 1].token()
                )
            })?);
        }
        Ok(vec)
    }

    fn function(&mut self) -> anyhow::Result<Function> {
        while match_token!(self, Token::EndLine) {}
        match_token_or_err!(self, Token::Func);
        if let Token::Literal(TokenLiteral::Identifier(ident)) = self.tokens[self.index].token() {
            self.index += 1;

            match_token_or_err!(self, Token::LParen);
            match_token_or_err!(self, Token::RParen);
            match_token_or_err!(self, Token::RArrow);

            self.match_type()?;
            let token = self.tokens[self.index - 1].token();

            match_token_or_err!(self, Token::EndLine);
            let mut block = self.block().with_context(|| {
                self.index -= 1;
                format!(
                    "From <function block >, past matched token: {}",
                    self.tokens[self.index - 1].token()
                )
            })?;
            //implicit return is void
            if block.last().is_none() || !matches!(block.last().unwrap(), Statement::Return(_)) {
                block.push(Statement::Return(None));
            }
            Ok(Function {
                name: ident,
                return_type: token,
                body: block,
            })
        } else {
            Err(anyhow!(
                "Expected Identifier! found {}\n{}",
                &self.tokens[self.index].token(),
                to_error!(self, &self.tokens[self.index])
            ))
        }
    }

    fn block(&mut self) -> Result<Vec<Statement>> {
        self.current_indentation_level += 1;
        let mut vec: Vec<Statement> = Vec::new();
        while !match_token!(self, Token::EndOfFile) {
            let mut indentation_level = 0;
            while match_token!(self, Token::Indent) {
                indentation_level += 1;
            }

            if indentation_level != self.current_indentation_level {
                self.index -= indentation_level + 1;
                println!("x{}", self.tokens[self.index].token());
                break;
            }

            vec.push(self.statement().with_context(|| {
                self.index -= 1;
                format!(
                    "From <block>, past matched token: {}",
                    self.tokens[self.index - 1].token()
                )
            })?);

            match_token_or_err!(self, Token::EndLine);
        }
        self.current_indentation_level -= 1;
        Ok(vec)
    }

    fn if_statement(&mut self) -> anyhow::Result<Statement> {
        let expr = self.expression().with_context(|| {
            self.index -= 1;
            format!(
                "From <if statement (conditional expression) >, past matched token: {}",
                self.tokens[self.index - 1].token()
            )
        })?;

        match_token_or_err!(self, Token::EndLine);

        let body = self.block().with_context(|| {
            self.index -= 1;
            format!(
                "From <if statement (true branch) >, past matched token: {}",
                self.tokens[self.index - 1].token()
            )
        })?;
        //println!("{}",self.tokens[self.index].token());
        let mut indent_level = 0;
        match_token_or_err!(self, Token::EndLine);
        //println!("{}",self.tokens[self.index].token());
        while match_token!(self, Token::Indent) {
            //dbg!(&self.tokens[self.index]);
            indent_level += 1;
        }
        //println!("indentlvl: {}, req {} token: {}",&indent_level,&self.current_indentation_level,&self.tokens[self.index].token());
        if indent_level == self.current_indentation_level {
            // println!("==");
            if match_token!(self, Token::Else) {
                //println!("else");
                match_token_or_err!(self, Token::EndLine);
                let body2 = self.block().with_context(|| {
                    self.index -= 1;
                    format!(
                        "From <if statement (false branch) >, past matched token: {}",
                        self.tokens[self.index - 1].token()
                    )
                })?;

                return Ok(Statement::If(expr, body, Some(body2)));
            } else {
                //println!("!else");
                self.index -= indent_level + 1;
                println!("2{}", self.tokens[self.index].token());
                Ok(Statement::If(expr, body, None))
            }
        } else {
            //println!("!=");
            self.index -= indent_level + 1;
            println!("3{}", self.tokens[self.index].token());
            Ok(Statement::If(expr, body, None))
        }
    }

    fn for_statement(&mut self) -> anyhow::Result<Statement> {
        let stmt = self.statement().with_context(|| {
            self.index -= 1;
            format!(
                "From <for statement (initialization statement) >, past matched token: {}",
                self.tokens[self.index - 1].token()
            )
        })?;
        match_token_or_err!(self, Token::Comma);
        let expr = self.expression().with_context(|| {
            self.index -= 1;
            format!(
                "From <for statement (comparison expression) >, past matched token: {}",
                self.tokens[self.index - 1].token()
            )
        })?;
        match_token_or_err!(self, Token::Comma);
        let stmt2 = self.statement().with_context(|| {self.index -= 1 ;format!("From <for statement (conditional incrementation statement) >, past matched token: {}",self.tokens[self.index-1].token())})?;
        match_token_or_err!(self, Token::EndLine);
        let body = self.block().with_context(|| {
            self.index -= 1;
            format!(
                "From <for statement (block) >, past matched token: {}",
                self.tokens[self.index - 1].token()
            )
        })?;
        Ok(Statement::For(
            Box::new(stmt),
            expr,
            Box::new(stmt2),
            Some(body),
        ))
    }

    fn statement(&mut self) -> anyhow::Result<Statement> {
        self.index += 1;
        let stmt = match self.tokens[self.index - 1].token() {
            Token::EndLine => {
                self.index -= 1;
                Ok(Statement::Null)
            }
            Token::For => self.for_statement().with_context(|| {
                self.index -= 1;
                format!(
                    "From <statement>, past matched token: {}",
                    self.tokens[self.index - 1].token()
                )
            }),
            Token::If => self.if_statement().with_context(|| {
                self.index -= 1;
                format!(
                    "From <statement>, past matched token: {}",
                    self.tokens[self.index - 1].token()
                )
            }),
            Token::Return => self.return_statement().with_context(|| {
                self.index -= 1;
                format!(
                    "From <statement>, past matched token: {}",
                    self.tokens[self.index - 1].token()
                )
            }),
            Token::Print => self.print_statement().with_context(|| {
                self.index -= 1;
                format!(
                    "From <statement>, past matched token: {}",
                    self.tokens[self.index - 1].token()
                )
            }),
            Token::Literal(TokenLiteral::Identifier(_)) => match self.tokens[self.index].token() {
                Token::Equals => self.assign_statement().with_context(|| {
                    self.index -= 1;
                    format!(
                        "From <statement>, past matched token: {}",
                        self.tokens[self.index - 1].token()
                    )
                }),
                Token::DColon => self.defvar_statement().with_context(|| {
                    self.index -= 1;
                    format!(
                        "From <statement>, past matched token: {}",
                        self.tokens[self.index - 1].token()
                    )
                }),
                _ => {
                    self.index -= 1;
                    self.expression_statement().with_context(|| {
                        self.index -= 1;
                        format!(
                            "From <statement>, past matched token: {}",
                            self.tokens[self.index - 1].token()
                        )
                    })
                }
            },
            _ => self.expression_statement().with_context(|| {
                self.index -= 1;
                format!(
                    "From <statement>, past matched token: {}",
                    self.tokens[self.index - 1].token()
                )
            }),
        };

        stmt
    }

    fn return_statement(&mut self) -> anyhow::Result<Statement> {
        if match_token!(self, Token::EndLine) {
            self.index -= 1;
            return Ok(Statement::Return(None));
        }
        let expr = self.expression().with_context(|| {
            self.index -= 1;
            format!(
                "From <return statement>, past matched token: {}",
                self.tokens[self.index - 1].token()
            )
        })?;

        Ok(Statement::Return(Some(expr)))
    }

    fn assign_statement(&mut self) -> anyhow::Result<Statement> {
        let variable = self.tokens[self.index - 1].token();
        let mut string_ident = String::new();

        if let Token::Literal(TokenLiteral::Identifier(ident)) = variable {
            string_ident = ident;
        } else {
            return Err(anyhow!(
                "Expected Identifier! found {}\n{}",
                &self.tokens[self.index].token(),
                to_error!(self, &self.tokens[self.index])
            ));
        }

        match_token_or_err!(self, Token::Equals);
        let expr = self.expression().with_context(|| {
            self.index -= 1;
            format!(
                "From <assign statement>, past matched token: {}",
                self.tokens[self.index - 1].token()
            )
        })?;
        Ok(Statement::Assign(string_ident, expr))
    }

    fn defvar_statement(&mut self) -> anyhow::Result<Statement> {
        let variable = self.tokens[self.index - 1].token();
        let mut string_ident = String::new();

        if let Token::Literal(TokenLiteral::Identifier(ident)) = variable {
            string_ident = ident;
        } else {
            return Err(anyhow!(
                "Expected Identifier! found {}\n{}",
                &self.tokens[self.index].token(),
                to_error!(self, &self.tokens[self.index])
            ));
        }
        self.index += 1;
        let typedata = self.match_type().with_context(|| {
            self.index -= 1;
            format!(
                "From <defvar statement>, past matched token: {}",
                self.tokens[self.index - 1].token()
            )
        })?;
        match_token_or_err!(self, Token::Equals);
        let expr = self.expression().with_context(|| {
            self.index -= 1;
            format!(
                "From <defvar statement>, past matched token: {}",
                self.tokens[self.index - 1].token()
            )
        })?;
        Ok(Statement::Declare(string_ident, typedata, expr))
    }

    fn expression_statement(&mut self) -> anyhow::Result<Statement> {
        let expr = self.expression().with_context(|| {
            self.index -= 1;
            format!(
                "From <expression statement>, past matched token: {}",
                self.tokens[self.index - 1].token()
            )
        })?;

        Ok(Statement::Expression(expr))
    }

    fn print_statement(&mut self) -> anyhow::Result<Statement> {
        //dbg!(&self.tokens[self.index]);
        let expr = self.expression().with_context(|| {
            self.index -= 1;
            format!(
                "From <print statement>, past matched token: {}",
                self.tokens[self.index - 1].token()
            )
        })?;
        //dbg!(&self.tokens[self.index]);

        Ok(Statement::Print(expr))
    }

    fn expression(&mut self) -> anyhow::Result<Expression> {
        self.equality()
    }

    fn equality(&mut self) -> anyhow::Result<Expression> {
        let mut expr = self.comparison().with_context(|| {
            self.index -= 1;
            format!(
                "From <equality expression>, past matched token: {}",
                self.tokens[self.index - 1].token()
            )
        })?;
        while match_token!(self, Token::EqualsEquals) {
            let op = self.tokens[self.index - 1].token();
            let rhs = self.comparison().with_context(|| {
                self.index -= 1;
                format!(
                    "From <equality expression>, past matched token: {}",
                    self.tokens[self.index - 1].token()
                )
            })?;
            expr = Expression::Binary(op.into(), Box::new(expr), Box::new(rhs));
        }
        Ok(expr)
    }

    fn comparison(&mut self) -> anyhow::Result<Expression> {
        let mut expr = self.term().with_context(|| {
            self.index -= 1;
            format!(
                "From <comparison expression>, past matched token: {}",
                self.tokens[self.index - 1].token()
            )
        })?;
        while match_token!(
            self,
            Token::Greater | Token::Lesser | Token::EqualsGreater | Token::EqualsLesser
        ) {
            let op = self.tokens[self.index - 1].token();
            let rhs = self.term().with_context(|| {
                self.index -= 1;
                format!(
                    "From <comparison expression>, past matched token: {}",
                    self.tokens[self.index - 1].token()
                )
            })?;
            expr = Expression::Binary(op.into(), Box::new(expr), Box::new(rhs));
        }
        Ok(expr)
    }

    fn term(&mut self) -> anyhow::Result<Expression> {
        let mut expr = self.factor().with_context(|| {
            self.index -= 1;
            format!(
                "From <add/sub expression>, past matched token: {}",
                self.tokens[self.index - 1].token()
            )
        })?;
        while match_token!(self, Token::Minus | Token::Plus) {
            let op = self.tokens[self.index - 1].token();
            let rhs = self.factor().with_context(|| {
                self.index -= 1;
                format!(
                    "From <add/sub expression>, past matched token: {}",
                    self.tokens[self.index - 1].token()
                )
            })?;
            expr = Expression::Binary(op.into(), Box::new(expr), Box::new(rhs));
        }
        Ok(expr)
    }

    fn factor(&mut self) -> anyhow::Result<Expression> {
        let mut expr = self.unary().with_context(|| {
            self.index -= 1;
            format!(
                "From <unary expression>, past matched token: {}",
                self.tokens[self.index - 1].token()
            )
        })?;
        while match_token!(self, Token::Star | Token::Slash) {
            let op = self.tokens[self.index - 1].token();
            let rhs = self.unary().with_context(|| {
                self.index -= 1;
                format!(
                    "From <factor expression>, past matched token: {}",
                    self.tokens[self.index - 1].token()
                )
            })?;
            expr = Expression::Binary(op.into(), Box::new(expr), Box::new(rhs));
        }
        Ok(expr)
    }

    fn unary(&mut self) -> anyhow::Result<Expression> {
        if match_token!(self, Token::Minus) {
            let op = self.tokens[self.index - 1].token();
            let rhs = self.unary().with_context(|| {
                self.index -= 1;
                format!(
                    "From <unary expression>, past matched token: {}",
                    self.tokens[self.index - 1].token()
                )
            })?;
            return Ok(Expression::Unary(ast::UnaryOpCode::NEG, Box::new(rhs)));
        }
        self.primary().with_context(|| {
            self.index -= 1;
            format!(
                "From <unary expression>, past matched token: {}",
                self.tokens[self.index - 1].token()
            )
        })
    }

    fn primary(&mut self) -> anyhow::Result<Expression> {
        match self.tokens[self.index].token() {
            Token::Literal(x) => {
                //dbg!(&self.tokens[self.index + 1]);
                self.index += 1;
                match self.tokens[self.index].token() {
                    Token::LBracket => {
                        self.index += 1;
                        if let TokenLiteral::Identifier(ident) = &x {
                            let expr = self.expression()?;
                            match_token_or_err!(self, Token::RBracket);
                            Ok(Expression::Access(ident.clone(), Box::new(expr)))
                        } else {
                            Err(anyhow!(
                                "Expected Identifier! found {}\n{}",
                                &self.tokens[self.index].token(),
                                to_error!(self, &self.tokens[self.index])
                            ))
                        }
                    }
                    Token::LParen => {
                        self.index += 1;
                        match_token_or_err!(self, Token::RParen);
                        if let TokenLiteral::Identifier(ident) = &x {
                            let expr = self.expression()?;
                            match_token_or_err!(self, Token::RBracket);
                            Ok(Expression::Call(ident.clone(), Vec::new()))
                        } else {
                            Err(anyhow!(
                                "Expected Identifier! found {}\n{}",
                                &self.tokens[self.index].token(),
                                to_error!(self, &self.tokens[self.index])
                            ))
                        }
                        
                    }
                    Token::RArrow => {
                        self.index += 1;
                        if let Token::Literal(x2) = &self.tokens[self.index].token() {
                            self.index += 1;
                            match (x, x2) {
                                (TokenLiteral::Value(i1), TokenLiteral::Value(i2)) => {
                                    match (&i1,i2) {
                                        (StaticValue::Integer(a), StaticValue::Integer(b)) => {
                                            Ok(Expression::Instance(
                                                Type::Object(ObjectType::Array(Box::new(Type::Int))),
                                                (*a..*b).map(|v| Expression::Literal(TokenLiteral::Value(StaticValue::Integer(v)))).collect(),
                                            ))
                                        }
                                        (StaticValue::Char(a), StaticValue::Char(b)) => {
                                            Ok(Expression::Instance(
                                                Type::Object(ObjectType::Array(Box::new(Type::Char))),
                                                (*a..*b).map(|v| Expression::Literal(TokenLiteral::Value(StaticValue::Char(v)))).collect(),
                                            ))
                                        }
                                        (_, _) => {
                                            self.index -= 1;
                                            return Err(anyhow!(
                                                "Incompatible types for list initailization!\n{}",
                                                to_error!(self, &self.tokens[self.index])
                                            ));
                                        }
                                    }
                                    
                                }
                                //(TokenLiteral::Float(i1), TokenLiteral::Float(i2)) => Ok(Expression::List( )),
                                (_, _) => {
                                    self.index -= 1;
                                    return Err(anyhow!(
                                        "Incompatible types for list initailization!\n{}",
                                        to_error!(self, &self.tokens[self.index])
                                    ));
                                }
                            }
                        } else {
                            return Err(anyhow!(
                                "Missing literals after the arrow!\n{}",
                                to_error!(self, &self.tokens[self.index])
                            ));
                        }
                    }
                    _ => Ok(Expression::Literal(x)),
                }
            }
            Token::LParen => {
                self.index += 1;
                if matches!(self.tokens[self.index + 1].token(), Token::Comma) {
                    let mut literals = Vec::new();
                    if let Token::Literal(lit) = &self.tokens[self.index].token() {
                        literals.push(lit.clone());
                        self.index += 1;
                    } else {
                        return Err(anyhow!(
                            "Expected a literal instead of {}\n{}",
                            &self.tokens[self.index].token(),
                            to_error!(self, &self.tokens[self.index])
                        ));
                    }
                    while match_token!(self, Token::Comma) {
                        if let Token::Literal(lit) = &self.tokens[self.index].token() {
                            literals.push(lit.clone());
                            self.index += 1;
                        } else {
                            return Err(anyhow!(
                                "Expected a literal instead of {}\n{}",
                                &self.tokens[self.index].token(),
                                to_error!(self, &self.tokens[self.index])
                            ));
                        }
                    }
                    match_token_or_err!(self, Token::RParen);
                    return Ok(Expression::Instance(Type::Int, literals.iter().map(|lit| Expression::Literal(lit.clone())).collect()));
                }
                let expr = self.expression()?;
                match_token_or_err!(self, Token::RParen);
                Ok(Expression::Grouping(Box::new(expr)))
            }
            x => Err(anyhow!(
                "Unexpected: <any primary expression> found {}\n{}",
                &x,
                to_error!(self, &self.tokens[self.index])
            )),
        }
    }
}
