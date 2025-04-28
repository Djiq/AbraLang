use std::{collections::VecDeque, iter::Peekable};

use crate::{frontend::tokenizer::TokenName, runtime::{types::{ObjectType, Type}, value::StaticValue}};

use super::{ast::{BinOpCode, Expression, Function, Item, Parameter, Statement, UnaryOpCode}, tokenizer::{Token, TokenLiteral}};

use anyhow::*;
type LexerItem = Result<(usize, Token, usize), anyhow::Error>;

pub struct Parser<L: Iterator<Item = LexerItem>> {
    lexer: Peekable<L>,
    buffer: VecDeque<(usize, Token, usize)>,
}

impl<L: Iterator<Item = LexerItem>> Parser<L> {
    pub fn new(lexer: L) -> Self {
        Parser {
            lexer: lexer.peekable(),
            buffer: VecDeque::with_capacity(2), // Lookahead buffer
        }
    }

    // --- Token Handling Helpers ---

    fn ensure_buffered(&mut self, n: usize) -> Result<()> {
        while self.buffer.len() < n {
            match self.lexer.next() {
                Some(Result::Ok(token_data)) => self.buffer.push_back(token_data),
                Some(Err(e)) => return Err(e), // Propagate lexer error
                None => break, // EOF
            }
        }
        Ok(())
    }

    fn peek_nth(&mut self, n: usize) -> Result<Option<&(usize, Token, usize)>> {
        self.ensure_buffered(n + 1)?;
        Ok(self.buffer.get(n))
    }

    fn peek_nth_token(&mut self, n: usize) -> Result<Option<&Token>> {
        Ok(self.peek_nth(n)?.map(|(_, token, _)| token))
    }

    fn consume(&mut self) -> Result<Option<(usize, Token, usize)>> {
        self.ensure_buffered(1)?;
        Ok(self.buffer.pop_front())
    }

    fn expect(&mut self, expected: Token) -> Result<(usize, Token, usize)> {
        let peeked_opt = self.peek_nth(0)?;
        if let Some((start, token, end)) = peeked_opt {
            if std::mem::discriminant(token) == std::mem::discriminant(&expected) {
                Ok(self.consume()?.unwrap()) // Safe unwrap due to peek
            } else {
                bail!("Expected token {} but found {:?} at {}..{}", expected.variant_name(), token.clone(), *start, *end)
            }
        } else {
            bail!("Expected token {} but found EOF", expected.variant_name())
        }
    }

    fn expect_identifier(&mut self) -> Result<(String, usize, usize)> {
        self.ensure_buffered(1)?;
        if let Some((start, token, end)) = self.buffer.front() {
            if matches!(token, Token::Literal(TokenLiteral::Identifier(_))) {
                match self.buffer.pop_front().unwrap() { // Safe unwrap
                    (s, Token::Literal(TokenLiteral::Identifier(name)), e) => Ok((name, s, e)),
                    _ => unreachable!(),
                }
            } else {
                let (start, consumed_token, end) = self.buffer.pop_front().unwrap();
                bail!("Expected Identifier but found {:?} at {}..{}", consumed_token, start, end)
            }
        } else {
            bail!("Expected Identifier but found EOF")
        }
    }

    fn consume_eols(&mut self) -> Result<()> {
        while self.peek_nth_token(0)? == Some(&Token::EndLine) {
            self.consume()?;
        }
        Ok(())
    }

    // --- Main Parsing Methods ---

    pub fn parse_program(&mut self) -> Result<Vec<Item>> {
        let mut items = Vec::new();
        self.consume_eols()?; // Consume leading EOLs
        while self.peek_nth_token(0)? != Some(&Token::EndOfFile) {
            items.push(self.parse_top_level_item()?);
            self.consume_eols()?; // Consume EOLs between items
        }
        self.expect(Token::EndOfFile)?;
        Ok(items)
    }

    fn parse_top_level_item(&mut self) -> Result<Item> {
        match self.peek_nth_token(0)? {
            Some(Token::Func) => self.parse_function().map(Item::Function),
            // Add other top-level items (struct, enum, etc.)
            Some(_) => {
                 let (start, unexpected_token, end) = self.consume()?.unwrap();
                 bail!("Expected top-level item (like 'func') but found {:?} at {}..{}", unexpected_token, start, end)
            },
            None => bail!("Expected top-level item but found EOF"),
        }
    }

    // --- Item/Structure Parsers ---

    fn parse_function(&mut self) -> Result<Function> {
        self.expect(Token::Func)?;
        let (name, _, _) = self.expect_identifier()?;
        self.expect(Token::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(Token::RParen)?;
        self.expect(Token::RArrow)?;
        let return_type = self.parse_type()?;
        let body = self.parse_statement_block()?; // Calls modified block parser
        Ok(Function { name, params, return_type, body })
    }

    fn parse_param_list(&mut self) -> Result<Vec<Parameter>> {
        let mut params = Vec::new();
        if self.peek_nth_token(0)? != Some(&Token::RParen) {
            loop {
                params.push(self.parse_parameter()?);
                if self.peek_nth_token(0)? != Some(&Token::Comma) { break; }
                self.consume()?; // Consume comma
            }
        }
        Ok(params)
    }

    fn parse_parameter(&mut self) -> Result<Parameter> {
        let (name, _, _) = self.expect_identifier()?;
        self.expect(Token::Colon)?;
        let ty = self.parse_type()?;
        Ok(Parameter { name, ty })
    }

    fn parse_type(&mut self) -> Result<Type> {
        let (start, token, end) = self.consume()?.ok_or_else(|| anyhow!("Expected type but found EOF"))?;
        match token {
            Token::Int => Ok(Type::Int), Token::Float => Ok(Type::Float),
            Token::Bool => Ok(Type::Bool), Token::Char => Ok(Type::Char),
            Token::String => Ok(Type::String),
            Token::LBracket => {
                let i = self.parse_type()?; self.expect(Token::RBracket)?;
                Ok(Type::Object(ObjectType::Array(Box::new(i))))
            },
            Token::Lesser => { // Assuming <K -> V> for Map
                let k = self.parse_type()?; self.expect(Token::RArrow)?;
                let v = self.parse_type()?; self.expect(Token::Greater)?;
                Ok(Type::Object(ObjectType::Map(Box::new(k), Box::new(v))))
            },
            // Add Token::Identifier for custom types if needed
            // Token::Literal(TokenLiteral::Identifier(name)) => Ok(Type::Custom(name)),
            other => bail!("Expected type but found {:?} at {}..{}", other, start, end),
        }
    }

    // --- Statement Parsing ---

    fn parse_statement_block(&mut self) -> Result<Vec<Statement>> {
        // Case 1: Single statement block (e.g., if x: print y)
        if self.peek_nth_token(0)? == Some(&Token::Colon) {
            self.consume()?; // Consume ':'
            // Expect a single statement rule, which MUST handle its own EOL
            let stmt = self.parse_statement_rule()?;
            Ok(vec![stmt])
        }
        // Case 2: Indented block
        else {
            self.consume_eols()?; // Consume EOLs before indent
            self.expect(Token::Indent)?;
            let mut stmts = Vec::new();
            while self.peek_nth_token(0)? != Some(&Token::Dedent) && self.peek_nth_token(0)? != Some(&Token::EndOfFile) {
                 // Each statement rule handles its own EOL
                 stmts.push(self.parse_statement_rule()?);
                 // Allow blank lines within the block
                 self.consume_eols()?;
            }

            if stmts.is_empty(){
                 let loc = match self.peek_nth(0)? {
                    Some((s,_,e)) => (*s, *e), None => (0,0),
                 };
                bail!("Indented block cannot be empty at {}..{}", loc.0, loc.1);
            }

            self.expect(Token::Dedent)?;
            // *** DO NOT expect EOL here *** - Handled by containing structure or main loop
            Ok(stmts)
        }
    }

    fn parse_statement_rule(&mut self) -> Result<Statement> {
        let first_token_peek = self.peek_nth_token(0)?.ok_or_else(|| anyhow!("Expected statement but found EOF"))?;

        // Dispatch based on the first token
        let statement = match first_token_peek {
            Token::Let => self.parse_let_statement(),
            Token::Return => self.parse_return_statement(),
            Token::Print => self.parse_print_statement(),
            Token::If => self.parse_if_statement(),
            Token::For => self.parse_for_statement(),
            // Add While, Loop, etc. here
            // Token::While => self.parse_while_statement(),

            Token::Literal(TokenLiteral::Identifier(_)) => {
                 // Lookahead for assignment
                 let is_assign = self.peek_nth_token(1)? == Some(&Token::Equals);
                 // Add lookahead for compound assignment if needed
                 // let is_compound_assign = matches!(self.peek_nth_token(1)?, Some(Token::PlusEquals | ...));

                 if is_assign /* || is_compound_assign */ {
                     self.parse_assignment_statement() // Or parse_compound_assignment()
                 } else {
                     // If not assignment, it's an expression statement (e.g., function call)
                     self.parse_expression_statement()
                 }
            }
            // Check if the token can start *any* valid expression
            _ if Self::is_start_of_expression(first_token_peek) => {
                 self.parse_expression_statement()
            }
            // Unexpected token
            _ => {
                let (start, token, end) = self.consume()?.unwrap(); // Consume to advance
                bail!("Expected statement start (Let, If, Identifier, etc.) but found {:?} at {}..{}", token, start, end)
            }
        };

        // The individual statement parsers called above are responsible
        // for consuming their trailing EOL. No extra EOL consumption needed here.
        statement
    }

    // --- Specific Statement Parsers (Each handles its own EOL) ---

    fn parse_let_statement(&mut self) -> Result<Statement> {
       self.expect(Token::Let)?; let (n,_,_) = self.expect_identifier()?; self.expect(Token::Colon)?;
       let t = self.parse_type()?; self.expect(Token::Equals)?; let e = self.parse_expression()?;
       self.expect(Token::EndLine)?; // Expect EOL
       Ok(Statement::Declare(n, t, e))
    }

    fn parse_return_statement(&mut self) -> Result<Statement> {
       self.expect(Token::Return)?;
       let e = if self.peek_nth_token(0)? != Some(&Token::EndLine) {
           Some(self.parse_expression()?)
       } else { None };
       self.expect(Token::EndLine)?; // Expect EOL
       Ok(Statement::Return(e))
    }

    fn parse_print_statement(&mut self) -> Result<Statement> {
       self.expect(Token::Print)?; let e = self.parse_expression()?;
       self.expect(Token::EndLine)?; // Expect EOL
       Ok(Statement::Print(e))
    }

    fn parse_assignment_statement(&mut self) -> Result<Statement> {
       let (n,_,_) = self.expect_identifier()?; self.expect(Token::Equals)?;
       let e = self.parse_expression()?;
       self.expect(Token::EndLine)?; // Expect EOL
       Ok(Statement::Assign(n, e))
    }
    // Add parse_compound_assignment if needed

    fn parse_expression_statement(&mut self) -> Result<Statement> {
       let e = self.parse_expression()?;
       self.expect(Token::EndLine)?; // Expect EOL
       Ok(Statement::Expression(e))
    }

    fn parse_if_statement(&mut self) -> Result<Statement> {
        self.expect(Token::If)?;
        let cond = self.parse_expression()?;
        let then_block = self.parse_statement_block()?; // Handles its own block end

        let else_block = if self.peek_nth_token(0)? == Some(&Token::Else) {
            self.consume()?; // Consume Else
             // Check for 'else if' vs 'else:'/'else <block>'
             if self.peek_nth_token(0)? == Some(&Token::If) {
                 // Parse 'else if' as a nested If statement wrapped in a block
                 let nested_if = self.parse_if_statement()?;
                 Some(vec![nested_if])
             } else {
                 // Parse 'else:' or 'else <indented block>'
                 Some(self.parse_statement_block()?) // Handles its own block end
             }
        } else { None };

        // No EOL expected here after blocks
        Ok(Statement::If(cond, then_block, else_block))
    }

    fn parse_for_statement(&mut self) -> Result<Statement> {
        self.expect(Token::For)?;
        let init = self.parse_for_init()?;    // Does not consume EOL
        self.expect(Token::Comma)?;
        let cond = self.parse_expression()?;  // Condition is just an expression
        self.expect(Token::Comma)?;
        let incr = self.parse_for_incr()?;    // Does not consume EOL
        let body = self.parse_statement_block()?; // Handles its own block end

        // No EOL expected here after block
        Ok(Statement::For(Box::new(init), cond, Box::new(incr), Some(body)))
    }

    // Helper for 'for' loop initializer (No EOL consumed)
    fn parse_for_init(&mut self) -> Result<Statement> {
         let first_token_peek = self.peek_nth_token(0)?.ok_or_else(|| anyhow!("Expected for loop initializer but found EOF"))?;
         match first_token_peek {
            Token::Let => { // let var: type = expr
                self.consume()?; // Consume Let
                let (n,_,_) = self.expect_identifier()?;
                self.expect(Token::Colon)?; let t = self.parse_type()?;
                self.expect(Token::Equals)?; let e = self.parse_expression()?;
                Ok(Statement::Declare(n,t,e)) // NO EOL
            }
            Token::Literal(TokenLiteral::Identifier(_)) => { // var = expr | expr
                 let is_assign = self.peek_nth_token(1)? == Some(&Token::Equals);
                 if is_assign { // Assignment: var = expr
                     let (n,_,_) = self.expect_identifier()?; self.expect(Token::Equals)?;
                     let e = self.parse_expression()?;
                     Ok(Statement::Assign(n,e)) // NO EOL
                 } else { // Just an expression (e.g., func_call())
                     let e = self.parse_expression()?; Ok(Statement::Expression(e)) // NO EOL
                 }
             }
             _ if Self::is_start_of_expression(first_token_peek) => { // Other expression
                let e = self.parse_expression()?; Ok(Statement::Expression(e)) // NO EOL
             }
             _ => {
                let (s,t,e) = self.peek_nth(0)?.unwrap();
                bail!("Expected for loop initializer (Let, Assignment, or Expression) but found {:?} at {}..{}", t.clone(), *s, *e)
             }
         }
    }

    // Helper for 'for' loop incrementor (No EOL consumed) - CORRECTED version
    fn parse_for_incr(&mut self) -> Result<Statement> {
        let first_token_peek = self.peek_nth_token(0)?.ok_or_else(|| anyhow!("Expected for loop incrementor but found EOF"))?;

        match first_token_peek {
            Token::Literal(TokenLiteral::Identifier(_)) => { // var = expr | expr
                 let is_assign = self.peek_nth_token(1)? == Some(&Token::Equals);
                 // Add compound assign check if needed
                 // let is_compound_assign = matches!(self.peek_nth_token(1)?, Some(Token::PlusEquals | ...));

                 if is_assign /* || is_compound_assign */ { // Assignment: var = expr
                     let (n,_,_) = self.expect_identifier()?;
                     self.expect(Token::Equals)?; // Or expect compound token
                     let e = self.parse_expression()?;
                     Ok(Statement::Assign(n,e)) // NO EOL
                     // Handle compound assignment Statement creation if needed
                 } else { // Just an expression
                     let e = self.parse_expression()?;
                     Ok(Statement::Expression(e)) // NO EOL
                 }
            }
            _ if Self::is_start_of_expression(first_token_peek) => { // Other expression
                let e = self.parse_expression()?;
                Ok(Statement::Expression(e)) // NO EOL
            }
            _ => {
                let (s,t,e) = self.peek_nth(0)?.unwrap();
                bail!("Expected for loop incrementor (Assignment or Expression) but found {:?} at {}..{}", t.clone(), *s, *e)
            }
        }
    }

    // --- Expression Parsing ---

    fn is_start_of_expression(token: &Token) -> bool {
         matches!(token, Token::Literal(_) | Token::LParen | Token::Minus | Token::Bang | Token::New | Token::LBracket /* Array lits? */ )
         // Add others like '{' for object literals if needed
    }

    fn parse_expression(&mut self) -> Result<Expression> {
        self.parse_equality() // Start with lowest precedence binary op handled
    }

    // Generic binary operator parsing (uses TryFrom for BinOpCode)
    fn parse_binary<F>(&mut self, parse_operand: F, operators: &[Token]) -> Result<Expression>
    where F: Fn(&mut Self) -> Result<Expression>
    {
        let mut left = parse_operand(self)?;
        while let Some(peeked_token_ref) = self.peek_nth_token(0)? {
             let peeked_token = peeked_token_ref; // Avoid borrow issues
             if operators.iter().any(|op| std::mem::discriminant(op) == std::mem::discriminant(peeked_token)) {
                 let (op_start, op_token, op_end) = self.consume()?.unwrap();

                 // Use TryFrom for conversion
                 let bin_op = BinOpCode::try_from(op_token.clone())
                     .map_err(|e| anyhow!("Internal parser error at {}..{}: {}", op_start, op_end, e))?;

                 let right = parse_operand(self)?;
                 left = Expression::Binary(bin_op, Box::new(left), Box::new(right));
             } else {
                 break;
             }
        }
        Ok(left)
    }

    // Operator Precedence Levels
    fn parse_equality(&mut self) -> Result<Expression> { self.parse_binary(Self::parse_comparison, &[Token::EqualsEquals, Token::BangEq]) }
    fn parse_comparison(&mut self) -> Result<Expression> { self.parse_binary(Self::parse_term, &[Token::Lesser, Token::Greater, Token::EqualsLesser, Token::EqualsGreater]) }
    fn parse_term(&mut self) -> Result<Expression> { self.parse_binary(Self::parse_factor, &[Token::Plus, Token::Minus]) }
    fn parse_factor(&mut self) -> Result<Expression> { self.parse_binary(Self::parse_unary, &[Token::Star, Token::Slash /* Add Token::Percent? */]) }

    // Unary Operators
    fn parse_unary(&mut self) -> Result<Expression> {
        if let Some(op_token) = self.peek_nth_token(0)? {
            let unary_op = match op_token {
                Token::Minus => Some(UnaryOpCode::NEG),
                Token::Bang  => Some(UnaryOpCode::NOT),
                 _           => None
            };
            if let Some(op) = unary_op {
                self.consume()?; // Consume operator
                let operand = self.parse_unary()?; // Parse operand recursively
                return Ok(Expression::Unary(op, Box::new(operand)));
            }
        }
        self.parse_postfix() // If no unary op, parse postfix
    }

    // Postfix Operators (Calls, Access)
    fn parse_postfix(&mut self) -> Result<Expression> {
        let mut expr = self.parse_primary()?;
        loop {
            match self.peek_nth_token(0)? {
                Some(Token::LBracket) => { // Array/Map Access: expr[index]
                    self.consume()?; // Consume '['
                    let index_expr = self.parse_expression()?;
                    self.expect(Token::RBracket)?;
                    // TODO: Generalize access beyond just identifiers if needed
                    match expr {
                        Expression::Literal(TokenLiteral::Identifier(name)) => {
                            expr = Expression::Access(name, Box::new(index_expr))
                        }
                        _ => bail!("Cannot apply index operator `[]` to this expression type: {}", expr),
                    }
                }
                Some(Token::LParen) => { // Function Call: expr(args)
                    self.consume()?; // Consume '('
                    let args = self.parse_call_args()?;
                    self.expect(Token::RParen)?;
                     // TODO: Generalize calls beyond just identifiers if needed (e.g., (get_func())() )
                    match expr {
                        Expression::Literal(TokenLiteral::Identifier(name)) => {
                            expr = Expression::Call(name, args)
                        }
                        _ => bail!("Cannot call this expression type like a function: {}", expr),
                    }
                }
                // Add Token::Dot for member access if needed
                _ => break, // No more postfix operators
            }
        }
        Ok(expr)
    }

    // Arguments for Calls/Instances
    fn parse_call_args(&mut self) -> Result<Vec<Expression>> {
        let mut args = Vec::new();
        if self.peek_nth_token(0)? != Some(&Token::RParen) {
             loop {
                 args.push(self.parse_expression()?);
                 if self.peek_nth_token(0)? != Some(&Token::Comma) { break; }
                 self.consume()?; // Consume ','
             }
        }
        Ok(args)
    }

    // Primary Expressions (Literals, Grouping, New, Identifiers)
    fn parse_primary(&mut self) -> Result<Expression> {
        let (start, token, end) = self.consume()?.ok_or_else(|| anyhow!("Expected primary expression but found EOF"))?;
        match token {
            Token::Literal(lit @ TokenLiteral::Value(_)) => {
                 // Check for Range Expression: literal -> literal
                 if self.peek_nth_token(0)? == Some(&Token::RArrow) {
                     self.parse_range_expression(lit, start, end)
                 } else {
                     Ok(Expression::Literal(lit)) // Simple literal
                 }
            }
            Token::Literal(lit @ TokenLiteral::Identifier(_)) => {
                // Identifier is initially parsed as a literal.
                // Postfix parsing will handle if it's used in a call or access.
                Ok(Expression::Literal(lit))
            }
            Token::LParen => { // Grouping: ( expr )
                let expr = self.parse_expression()?;
                self.expect(Token::RParen)?;
                Ok(Expression::Grouping(Box::new(expr)))
            }
            Token::New => { // Instance Creation: new Type(args)
                let ty = self.parse_type()?;
                self.expect(Token::LParen)?;
                let args = self.parse_call_args()?;
                self.expect(Token::RParen)?;
                Ok(Expression::Instance(ty, args))
            }
             // Add Token::LBracket for array literals if needed
             // Add Token::LBrace for object/struct literals if needed
            other => bail!("Expected primary expression (Literal, Identifier, '(', 'new') but found {:?} at {}..{}", other, start, end),
        }
    }

    // Range Expression: literal -> literal (creates an array instance)
     fn parse_range_expression(&mut self, start_lit: TokenLiteral, start_loc: usize, _end_loc: usize) -> Result<Expression> {
         self.expect(Token::RArrow)?; // Consume '->'

         let (end_start, end_token, end_end) = self.consume()?.ok_or_else(|| anyhow!("Expected end of range expression after '->' but found EOF"))?;
         let end_lit = match end_token {
             Token::Literal(l @ TokenLiteral::Value(_)) => l,
             o => bail!("Expected literal value for end of range but found {:?} at {}..{}", o, end_start, end_end),
         };

         match (start_lit, end_lit) {
            (TokenLiteral::Value(StaticValue::Integer(s)), TokenLiteral::Value(StaticValue::Integer(e))) => {
                if s >= e { bail!("Range start {} must be less than end {} at {}..{}", s, e, start_loc, end_end); }
                 let elements = (s..e).map(|v| Expression::Literal(TokenLiteral::Value(StaticValue::Integer(v)))).collect();
                 Ok(Expression::Instance(Type::Object(ObjectType::Array(Box::new(Type::Int))), elements))
             }
            (TokenLiteral::Value(StaticValue::Char(s)), TokenLiteral::Value(StaticValue::Char(e))) => {
                 if s > e { bail!("Range start '{}' must be less than or equal to end '{}' at {}..{}", s, e, start_loc, end_end); }
                 let elements = (s..=e).map(|v| Expression::Literal(TokenLiteral::Value(StaticValue::Char(v)))).collect();
                 Ok(Expression::Instance(Type::Object(ObjectType::Array(Box::new(Type::Char))), elements))
            }
            (l, r) => bail!("Cannot create a range between {:?} and {:?} starting near {}", l, r, start_loc),
         }
     }

} // end impl Parser