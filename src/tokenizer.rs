use serde::*;
use crate::*;

pub struct TokenData{
    pub line: usize,
    pub character: usize,
    pub from: String,
    pub token: Token
}


impl TokenData{
    fn new<T: Into<String>>(line: usize, character: usize, from: T,token: Token) -> Self{
        TokenData{
            line,
            character,
            from: from.into(),
            token
        }
    }

    pub fn token(&self) -> Token{
        self.token.clone()
    }
}



macro_rules! match_token{
    ($self:ident,$pattern:pat $(if $guard:expr)? $(,)?) => {
        if !matches!($self.tokens[$self.index],$pattern){
            false
        } else {
            $self.index += 1;
            true
        }
    };
}


#[derive(Debug,Clone,Deserialize,Serialize,PartialEq, PartialOrd)]
pub enum Token{
    DColonDColon,
    DColon,
    Comma,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Plus,
    PlusEquals,
    Minus,
    MinusEquals,
    Slash,
    SlashEquals,
    Star,
    StarEquals,
    Equals,
    EqualsEquals,
    Greater,
    Lesser,
    EqualsGreater,
    EqualsLesser,
    RArrow,
    LArrow,
    EndLine,
    Indent,
    Bang,
    BangEq,

    Literal(TokenLiteral),
    Func,
    Int,
    Float,
    Char,
    Bool,
    String,
    Print,
    Return,
    If,
    Else,
    For,
    While,
    Do,
    Loop,
    New,
    EndOfFile,
}

impl Display for Token{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_string(self).unwrap();
        write!(f,"{}",&s)
    }
}


#[derive(Debug,Clone,Deserialize,Serialize,PartialEq, PartialOrd)]
pub enum TokenLiteral{
    Identifier(String),
    Integer(isize),
    Float(f64),
    Bool(bool),
    Char(char),
    String(String)
}

impl Display for TokenLiteral{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_string(self).unwrap();
        write!(f,"{}",&s)
    }
}

pub fn tokenize(file:String) -> Result<Vec<TokenData>> {
    let mut errors : Vec<anyhow::Error> = Vec::new();
    let mut v = file.lines().enumerate().flat_map(|line|parse_line(line.1,line.0).map_err(|err| errors.push(err))).flatten().collect::<Vec<TokenData>>();
    if errors.is_empty() {
        v.push(TokenData::new(file.len(), file.len(), "", Token::EndOfFile));
        Ok(v)
    } else {
        for error in errors {
            println!("Tokenization Error : {:?}",error);
        }
        Err(anyhow!("Tokenization Error!"))
    }
}

fn parse_line(line: &str,line_num: usize) -> Result<Vec<TokenData>> {
    let mut line_str = line.to_owned();
    let mut indentation_level = 0;
    let errors : Vec<anyhow::Error> = Vec::new();
    loop{
        if line_str.starts_with("    "){
            line_str = line_str.strip_prefix("    ").unwrap().to_owned();
            indentation_level += 1;
            continue;
        }
        if line_str.starts_with("\t"){
            line_str = line_str.strip_prefix("\t").unwrap().to_owned();
            indentation_level += 1;
            continue;
        }
        break;
    }
    let mut ret : Vec<TokenData> = (0..indentation_level).map(|x| TokenData::new(line_num, x, "\t", Token::Indent)).collect();
    let mut iter = line.chars().enumerate().peekable();
    let mut o_char= iter.next();

    
   
    while o_char.is_some() {
        let char = o_char.unwrap().1;
        macro_rules! token_to_tokendata {
            ($token:expr) => {
                TokenData::new(line_num,o_char.unwrap().0,String::from(char),$token)
            };
        }
        macro_rules! token_to_tokendata_string {
            ($token:expr,$string:expr) => {
                TokenData::new(line_num,o_char.unwrap().0,$string.clone(),$token)
            };
        }
        match char {
            ']' => ret.push(token_to_tokendata!(Token::RBracket)),
            '[' => ret.push(token_to_tokendata!(Token::LBracket)),
            ',' => ret.push(token_to_tokendata!(Token::Comma)),
            ':' => {
                if iter.peek().unwrap_or(&(line.len(),' ')).1 == ':' {
                    ret.push(token_to_tokendata!(Token::DColonDColon));
                    iter.next().unwrap();
                } else {
                    ret.push(token_to_tokendata!(Token::DColon));
                }
            }
            '+' => {
                if iter.peek().unwrap_or(&(line.len(),' ')).1 == '=' {
                    ret.push(token_to_tokendata!(Token::PlusEquals));
                    iter.next().unwrap();
                } else {
                    ret.push(token_to_tokendata!(Token::Plus));
                }
            }
            '-' => {
                let c = iter.peek().unwrap_or(&(line.len(),' ')).1;
                if c == '=' {
                    ret.push(token_to_tokendata!(Token::MinusEquals));
                    iter.next().unwrap();
                } else if c == '>' {
                    ret.push(token_to_tokendata!(Token::RArrow));
                    iter.next().unwrap();
                }else {
                    ret.push(token_to_tokendata!(Token::Minus));
                }
            }
            '/' => {
                let c = iter.peek().unwrap_or(&(line.len(),' ')).1;
                if c == '/'{
                    while iter.peek().unwrap_or(&(line.len(),'\n')).1 != '\n' {
                        iter.next();
                    }
                }else if c == '=' {
                    ret.push(token_to_tokendata!(Token::SlashEquals));
                    iter.next().unwrap();
                } else {
                    ret.push(token_to_tokendata!(Token::Slash));
                }
            }
            '=' => {
                let c = iter.peek().unwrap_or(&(line.len(),' ')).1;
                if c == '=' {
                    ret.push(token_to_tokendata!(Token::EqualsEquals));
                    iter.next().unwrap();
                } else {
                    ret.push(token_to_tokendata!(Token::Equals));
                }
            }
            '>' => {
                let c =iter.peek().unwrap_or(&(line.len(),' ')).1;
                if c == '=' {
                    ret.push(token_to_tokendata!(Token::EqualsGreater));
                    iter.next().unwrap();
                } else {
                    ret.push(token_to_tokendata!(Token::Greater));
                }
            }
            '<' => {
                let c = iter.peek().unwrap_or(&(line.len(),' ')).1;
                if c == '=' {
                    ret.push(token_to_tokendata!(Token::EqualsLesser));
                    iter.next().unwrap();
                } else if c == '-' {
                    ret.push(token_to_tokendata!(Token::LArrow));
                    iter.next().unwrap();
                }else {
                    ret.push(token_to_tokendata!(Token::Lesser));
                }
            }
            '(' => ret.push(token_to_tokendata!(Token::LParen)),
            ')' => ret.push(token_to_tokendata!(Token::RParen)),
            '\'' => {
                let ch = iter.next().unwrap().1;
                match ch {
                    '\\' => {
                        let next_char = iter.next().unwrap().1;
                        match next_char {
                            'n' => ret.push(token_to_tokendata!(Token::Literal(TokenLiteral::Char('\n')))), 
                            _ => println!("{}",next_char),
                        }
                        iter.next();
                    }
                    x => ret.push(token_to_tokendata!(Token::Literal(TokenLiteral::Char(x)))),
                }
                iter.next();
            }
            '"' => {
                let mut s = String::new();
                while iter.peek().is_some() && iter.peek().unwrap_or(&(line.len(),'"')).1 != '"' {
                    let cha = iter.next().unwrap().1;
                    match cha {
                        '\\' => {
                            let cha2 = iter.next().unwrap().1;
                            match cha2 {
                                'n' => s.push('\n'),
                                't' => s.push('\n'),

                                t2 => {
                                    s.push(cha);
                                    s.push(t2);
                                },
                            }
                        }
                        t => s.push(t),
                    }
                    
                }
                iter.next();
                ret.push(token_to_tokendata_string!(Token::Literal(TokenLiteral::String(s)),s));
            }
            c => {
                if c ==' ' || c =='\t' {
                    o_char = iter.next();
                    continue;
                }
                let mut s = String::new();
                s.push(c);
                if c.is_numeric() {
                    let mut is_float = false;
                    while iter.peek().is_some() && (iter.peek().unwrap().1.is_numeric() || (iter.peek().unwrap_or(&(line.len(),' ')).1 == '.' && !is_float)) {
                        let ch = iter.next().unwrap().1;
                        if ch == '.' {
                            is_float = true;
                        }
                        s.push(ch);
                    }
                    if is_float{
                        ret.push(token_to_tokendata_string!(Token::Literal(TokenLiteral::Float(s.parse().ok().ok_or(anyhow!("Parsing error"))?)),s));
                    } else {
                        ret.push(token_to_tokendata_string!(Token::Literal(TokenLiteral::Integer(s.parse().ok().ok_or(anyhow!("Parsing error"))?)),s));
                    }
                } else {

                    while iter.peek().is_some() && (iter.peek().unwrap().1.is_ascii_alphanumeric() || iter.peek().unwrap_or(&(line.len(),' ')).1 == '_')  {
                        let ch = iter.next().unwrap().1;
                        s.push(ch);
                    }
                    

                    match s.as_str() {
                        "func" => ret.push(token_to_tokendata_string!(Token::Func,s)),
                        "int" => ret.push(token_to_tokendata_string!(Token::Int,s)),
                        "float" => ret.push(token_to_tokendata_string!(Token::Float,s)),
                        "true" => ret.push(token_to_tokendata_string!(Token::Literal(TokenLiteral::Bool(true)),s)),
                        "false" => ret.push(token_to_tokendata_string!(Token::Literal(TokenLiteral::Bool(false)),s)),
                        "bool" => ret.push(token_to_tokendata_string!(Token::Bool,s)),
                        "print" => ret.push(token_to_tokendata_string!(Token::Print,s)),
                        "return" => ret.push(token_to_tokendata_string!(Token::Return,s)),
                        "if" => ret.push(token_to_tokendata_string!(Token::If,s)),
                        "else" => ret.push(token_to_tokendata_string!(Token::Else,s)),
                        "for" => ret.push(token_to_tokendata_string!(Token::For,s)),
                        "while" => ret.push(token_to_tokendata_string!(Token::While,s)),
                        "do" => ret.push(token_to_tokendata_string!(Token::Do,s)),
                        "loop" => ret.push(token_to_tokendata_string!(Token::Loop,s)),
                        "new" => ret.push(token_to_tokendata_string!(Token::New,s))
                        _ => ret.push(token_to_tokendata_string!(Token::Literal(TokenLiteral::Identifier(s)),s)),
                    }

                    }
                }
            }

            o_char = iter.next();
        }


    ret.push(TokenData::new(line_num, line.len(), "", Token::EndLine));
    if errors.is_empty() {
        Ok(ret)
    } else {
        Err(anyhow!("Errors encountered during tokenization"))
    }

    //Err(ParsingError::Error)
}
