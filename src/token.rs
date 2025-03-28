use std::{fmt::Display, iter::Peekable, str::CharIndices};

use serde::{Deserialize, Serialize};
use anyhow::{anyhow, Result};
use crate::value::StaticValue;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, PartialOrd)]
pub enum Token {
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
    Dedent,
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

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_string(self).unwrap();
        write!(f, "{}", &s)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, PartialOrd)]
pub enum TokenLiteral {
    Identifier(String),
    Value(StaticValue)
}

impl TokenLiteral {
    pub fn to_static_value(&self) -> StaticValue {
        match self {
            TokenLiteral::Identifier(_) => StaticValue::Null,
            TokenLiteral::Value(val) => val.clone()
        }
    }
}

impl Display for TokenLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_string(self).unwrap();
        write!(f, "{}", &s)
    }
}

pub struct Tokenizer<'i> {
    input: &'i str,
    characters: Peekable<CharIndices<'i>>,
    emitted_eof: bool,
    indent_stack: Vec<usize>,
    needs_indent_check: bool,
    pending_dedents: usize,
    current_token_start_pos: usize,
}

const SPACES_PER_INDENT: usize = 4;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
enum IndentStyle {
    Undetermined,
    Spaces,
    Tabs,
}


impl<'a> Tokenizer<'a> {
    pub fn new(input: &'a str) -> Tokenizer<'a> {
        Tokenizer {
            input,
            characters: input.char_indices().peekable(),
            emitted_eof: false,
            indent_stack: vec![0],
            needs_indent_check: true,
            pending_dedents: 0,
            current_token_start_pos: 0,
        }
    }

    fn consume_while<F>(&mut self, start_index: usize, condition: F) -> (usize, &'a str)
    where
        F: Fn(char) -> bool,
    {
        let mut current_idx = start_index;
        let mut end_idx = start_index;

        if let Some(&(idx, _)) = self.characters.peek() {
             current_idx = idx;
             end_idx = idx;
        } else {
            return (start_index, &self.input[start_index..start_index]);
        }


        while let Some(&(idx, ch)) = self.characters.peek() {
            if condition(ch) {
                end_idx = idx + ch.len_utf8();
                self.characters.next();
            } else {
                break;
            }
        }
        (end_idx, &self.input[start_index..end_idx])
    }

    fn consume_identifier(&mut self, start_index: usize, first_char: char) -> (usize, Token, usize) {
        let text_start_index = start_index + first_char.len_utf8();
        let (end_index, text) = self.consume_while(text_start_index, |c| c.is_ascii_alphanumeric() || c == '_');
        let full_id = format!("{}{}", first_char, text);

        let token = match full_id.as_str() {
            "func" => Token::Func,
            "int" => Token::Int,
            "float" => Token::Float,
            "char" => Token::Char,
            "bool" => Token::Bool,
            "string" => Token::String,
            "print" => Token::Print,
            "return" => Token::Return,
            "if" => Token::If,
            "else" => Token::Else,
            "for" => Token::For,
            "while" => Token::While,
            "do" => Token::Do,
            "loop" => Token::Loop,
            "new" => Token::New,
            "true" => Token::Literal(TokenLiteral::Value(StaticValue::Bool(true))),
            "false" => Token::Literal(TokenLiteral::Value(StaticValue::Bool(false))),
            _ => Token::Literal(TokenLiteral::Identifier(full_id)),
        };

        (start_index, token, end_index)
    }


    fn consume_number(&mut self, start_index: usize, first_char: char) -> Result<(usize, Token, usize)> {
        let mut end_index = start_index + first_char.len_utf8();
        let mut is_float = false;
        let mut num_str_buf = String::with_capacity(10);
        num_str_buf.push(first_char);

        while let Some(&(idx, ch)) = self.characters.peek() {
            if ch.is_ascii_digit() {
                 end_index = idx + ch.len_utf8();
                 num_str_buf.push(ch);
                 self.characters.next();
            } else {
                break;
            }
        }

        if let Some(&(idx_dot, '.')) = self.characters.peek() {
            let mut ahead_peek = self.characters.clone();
            ahead_peek.next();
            if ahead_peek.peek().map_or(false, |&(_, c)| c.is_ascii_digit()) {
                is_float = true;
                self.characters.next();
                end_index = idx_dot + '.'.len_utf8();
                num_str_buf.push('.');

                while let Some(&(idx_frac, ch_frac)) = self.characters.peek() {
                    if ch_frac.is_ascii_digit() {
                         end_index = idx_frac + ch_frac.len_utf8();
                         num_str_buf.push(ch_frac);
                         self.characters.next();
                    } else {
                        break;
                    }
                }
            }
        }

        let number_str = num_str_buf.as_str();

        if is_float {
            match number_str.parse::<f64>() {
                Ok(f) => Ok((start_index, Token::Literal(TokenLiteral::Value(StaticValue::Float(f))), end_index)),
                Err(e) => Err(anyhow!("Invalid float literal '{}' at index {}: {}", number_str, start_index, e)),
            }
        } else {
             match number_str.parse::<i64>() {
                Ok(i) => Ok((start_index, Token::Literal(TokenLiteral::Value(StaticValue::Integer(i))), end_index)),
                Err(e) => Err(anyhow!("Invalid integer literal '{}' at index {}: {}", number_str, start_index, e)),
            }
        }
    }

    fn consume_string(&mut self, start_index: usize) -> Result<(usize, Token, usize)> {
        let mut content = String::new();
        let mut current_idx = start_index + '"'.len_utf8();

        loop {
            match self.characters.next() {
                Some((idx, '"')) => {
                    let end_index = idx + '"'.len_utf8();
                    return Ok((start_index, Token::Literal(TokenLiteral::Value(StaticValue::String(content))), end_index));
                }
                Some((idx, '\\')) => {
                     current_idx = idx + '\\'.len_utf8();
                     match self.characters.next() {
                         Some((idx_esc, 'n')) => { content.push('\n'); current_idx = idx_esc + 'n'.len_utf8(); },
                         Some((idx_esc, 't')) => { content.push('\t'); current_idx = idx_esc + 't'.len_utf8(); },
                         Some((idx_esc, '\\')) => { content.push('\\'); current_idx = idx_esc + '\\'.len_utf8(); },
                         Some((idx_esc, '"')) => { content.push('"'); current_idx = idx_esc + '"'.len_utf8(); },
                         Some((idx_esc, other)) => {
                             return Err(anyhow!("Invalid escape sequence '\\{}' in string literal starting at index {}", other, idx));
                         }
                         None => {
                            return Err(anyhow!("Unterminated string literal starting at index {}", start_index));
                         }
                     }
                }
                Some((idx, ch)) => {
                    content.push(ch);
                    current_idx = idx + ch.len_utf8();
                }
                None => {
                    return Err(anyhow!("Unterminated string literal starting at index {}", start_index));
                }
            }
        }
    }

     fn consume_char(&mut self, start_index: usize) -> Result<(usize, Token, usize)> {
        let char_val: char;
        let pos_after_char: usize;

        match self.characters.next() {
            Some((idx, '\\')) => {
                match self.characters.next() {
                    Some((idx_esc, 'n')) => { char_val = '\n'; pos_after_char = idx_esc + 'n'.len_utf8(); },
                    Some((idx_esc, 't')) => { char_val = '\t'; pos_after_char = idx_esc + 't'.len_utf8(); },
                    Some((idx_esc, '\\')) => { char_val = '\\'; pos_after_char = idx_esc + '\\'.len_utf8(); },
                    Some((idx_esc, '\'')) => { char_val = '\''; pos_after_char = idx_esc + '\''.len_utf8(); },
                    Some((idx_esc, other)) => {
                        return Err(anyhow!("Invalid escape sequence '\\{}' in char literal at index {}", other, idx));
                    }
                    None => {
                        return Err(anyhow!("Unterminated char literal (EOF after escape) starting at index {}", start_index));
                    }
                }
            }
            Some((idx, '\'')) => {
                 return Err(anyhow!("Empty char literal at index {}", start_index));
            }
            Some((idx, ch)) => {
                char_val = ch;
                pos_after_char = idx + ch.len_utf8();
            }
            None => {
                return Err(anyhow!("Unterminated char literal (EOF after opening quote) starting at index {}", start_index));
            }
        }

        match self.characters.next() {
            Some((idx_close, '\'')) => {
                let end_index = idx_close + '\''.len_utf8();
                 Ok((start_index, Token::Literal(TokenLiteral::Value(StaticValue::Char(char_val))), end_index))
            }
            Some((idx_bad, other)) => Err(anyhow!("Expected closing ' for char literal at index {}, found '{}'", pos_after_char, other)),
            None => Err(anyhow!("Unterminated char literal (EOF before closing quote) starting at index {}", start_index)),
        }
    }


    fn calculate_indent_level(&mut self) -> Result<(usize, usize)> {
        let mut level = 0;
        let mut style = IndentStyle::Undetermined;
        let mut space_count = 0;
        let mut start_pos = self.characters.peek().map_or(self.input.len(), |(idx, _)| *idx);
        let mut pos_after_indent = start_pos;

        loop {
            match self.characters.peek() {
                Some(&(idx, ' ')) => {
                    pos_after_indent = idx + ' '.len_utf8();
                    match style {
                        IndentStyle::Undetermined => {
                            style = IndentStyle::Spaces;
                            space_count = 1;
                            self.characters.next();
                        }
                        IndentStyle::Spaces => {
                            space_count += 1;
                            self.characters.next();
                            if space_count == SPACES_PER_INDENT {
                                level += 1;
                                space_count = 0;
                            }
                        }
                        IndentStyle::Tabs => {
                             let err_pos = idx;
                            return Err(anyhow!(
                                "Mixed indentation: Found space at index {} after using tabs for indentation on this line.", err_pos
                            ));
                        }
                    }
                }
                Some(&(idx, '\t')) => {
                     pos_after_indent = idx + '\t'.len_utf8();
                    match style {
                        IndentStyle::Undetermined => {
                            style = IndentStyle::Tabs;
                            level += 1;
                            self.characters.next();
                        }
                        IndentStyle::Spaces => {
                            let err_pos = idx;
                            return Err(anyhow!(
                                "Mixed indentation: Found tab at index {} after using spaces for indentation on this line.", err_pos
                            ));
                        }
                        IndentStyle::Tabs => {
                            level += 1;
                            self.characters.next();
                        }
                    }
                }
                _ => {
                    break;
                }
            }
        }

        if style == IndentStyle::Spaces && space_count != 0 {
            return Err(anyhow!(
                "Inconsistent indentation: Found {} spaces at index {} which is not a multiple of {}.",
                space_count, pos_after_indent - space_count, SPACES_PER_INDENT
            ));
        }

        Ok((level, pos_after_indent))
    }
}

pub type Spanned<Tok, Loc, Error> = Result<(Loc, Tok, Loc), Error>;

impl<'i> Iterator for Tokenizer<'i> {
    type Item = Spanned<Token, usize, anyhow::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pending_dedents > 0 {
            self.pending_dedents -= 1;
            let pos = self.current_token_start_pos;
            return Some(Ok((pos, Token::Dedent, pos)));
        }

        loop {
            if self.needs_indent_check {
                self.needs_indent_check = false;

                 let indent_start_pos = self.characters.peek().map_or(self.input.len(), |(idx, _)| *idx);
                 self.current_token_start_pos = indent_start_pos;


                let (current_level, pos_after_indent) = match self.calculate_indent_level() {
                    Ok(result) => result,
                    Err(e) => return Some(Err(e)),
                };
                self.current_token_start_pos = pos_after_indent;


                 let is_blank_or_comment = match self.characters.peek() {
                    None => true,
                    Some(&(_, '\n')) => true,
                    Some(&(_, '/')) => {
                         let mut ahead_peek = self.characters.clone();
                         ahead_peek.next();
                         ahead_peek.peek().map_or(false, |&(_, c)| c == '/')
                    }
                    _ => false,
                 };

                 if is_blank_or_comment {
                      if let Some(&(_, '\n')) = self.characters.peek() {
                            self.needs_indent_check = true;
                      }
                      continue;
                 }


                let last_level = *self.indent_stack.last().unwrap();

                if current_level > last_level {
                    if current_level == last_level + 1 {
                        self.indent_stack.push(current_level);
                        return Some(Ok((indent_start_pos, Token::Indent, indent_start_pos)));
                    } else {
                        return Some(Err(anyhow!(
                            "Invalid indentation: Indented to level {} from level {} at index {}. Can only indent one level at a time.",
                            current_level, last_level, indent_start_pos
                        )));
                    }
                } else if current_level < last_level {
                    while *self.indent_stack.last().unwrap() > current_level {
                        self.indent_stack.pop();
                        self.pending_dedents += 1;
                    }

                    if *self.indent_stack.last().unwrap() != current_level {
                         return Some(Err(anyhow!(
                            "Inconsistent indentation: Dedented to level {} at index {}, which does not match any previous indentation level. Known levels: {:?}",
                            current_level, indent_start_pos, self.indent_stack
                        )));
                    }

                    if self.pending_dedents > 0 {
                        self.pending_dedents -= 1;
                        let pos = self.current_token_start_pos;
                        return Some(Ok((pos, Token::Dedent, pos)));
                    }
                }
            }

            let peeked_char = self.characters.peek();

            match peeked_char {
                None => {
                    if !self.emitted_eof {
                        let eof_pos = self.input.len();
                        self.current_token_start_pos = eof_pos;
                        while *self.indent_stack.last().unwrap() > 0 {
                             self.indent_stack.pop();
                             self.pending_dedents += 1;
                        }
                        if self.pending_dedents > 0 {
                            self.pending_dedents -= 1;
                            return Some(Ok((eof_pos, Token::Dedent, eof_pos)));
                        }

                        self.emitted_eof = true;
                        return Some(Ok((eof_pos, Token::EndOfFile, eof_pos)));
                    } else {
                        return None;
                    }
                }

                Some(&(_, ch)) if ch.is_whitespace() && ch != '\n' => {
                    self.characters.next();
                    continue;
                }

                Some(&(idx, '\n')) => {
                    self.characters.next();
                    self.needs_indent_check = true;
                    return Some(Ok((idx, Token::EndLine, idx + 1)));
                }

                Some(&(start_index, current_char)) => {
                    self.current_token_start_pos = start_index;
                    self.characters.next();
                    let end_index = start_index + current_char.len_utf8();

                    let result = match current_char {
                        '(' => Ok((start_index, Token::LParen, end_index)),
                        ')' => Ok((start_index, Token::RParen, end_index)),
                        '[' => Ok((start_index, Token::LBracket, end_index)),
                        ']' => Ok((start_index, Token::RBracket, end_index)),
                        ',' => Ok((start_index, Token::Comma, end_index)),
                         ':' => {
                             if self.characters.peek().map(|&(_, c)| c == ':').unwrap_or(false) {
                                self.characters.next();
                                Ok((start_index, Token::DColonDColon, start_index + 2))
                            } else {
                                Ok((start_index, Token::DColon, end_index))
                            }
                         }
                        '+' => {
                            if self.characters.peek().map(|&(_, c)| c == '=').unwrap_or(false) {
                                self.characters.next();
                                Ok((start_index, Token::PlusEquals, start_index + 2))
                            } else {
                                Ok((start_index, Token::Plus, end_index))
                            }
                        }
                         '-' => {
                            if self.characters.peek().map(|&(_, c)| c == '=').unwrap_or(false) {
                                self.characters.next();
                                Ok((start_index, Token::MinusEquals, start_index + 2))
                            } else if self.characters.peek().map(|&(_, c)| c == '>').unwrap_or(false) {
                                self.characters.next();
                                Ok((start_index, Token::RArrow, start_index + 2))
                            } else {
                                Ok((start_index, Token::Minus, end_index))
                            }
                        }
                        '*' => {
                             if self.characters.peek().map(|&(_, c)| c == '=').unwrap_or(false) {
                                self.characters.next();
                                Ok((start_index, Token::StarEquals, start_index + 2))
                            } else {
                                Ok((start_index, Token::Star, end_index))
                            }
                        }
                         '/' => {
                             if self.characters.peek().map(|&(_, c)| c == '=').unwrap_or(false) {
                                self.characters.next();
                                Ok((start_index, Token::SlashEquals, start_index + 2))
                            } else if self.characters.peek().map(|&(_, c)| c == '/').unwrap_or(false) {
                                self.characters.next();
                                let comment_start = start_index + 2;
                                let (_comment_end, _) = self.consume_while(comment_start, |c| c != '\n');
                                continue;
                            }
                            else {
                                Ok((start_index, Token::Slash, end_index))
                            }
                        }
                         '=' => {
                            if self.characters.peek().map(|&(_, c)| c == '=').unwrap_or(false) {
                                self.characters.next();
                                Ok((start_index, Token::EqualsEquals, start_index + 2))
                            } else {
                                Ok((start_index, Token::Equals, end_index))
                            }
                        }
                         '>' => {
                            if self.characters.peek().map(|&(_, c)| c == '=').unwrap_or(false) {
                                self.characters.next();
                                Ok((start_index, Token::EqualsGreater, start_index + 2))
                            } else {
                                Ok((start_index, Token::Greater, end_index))
                            }
                        }
                        '<' => {
                            if self.characters.peek().map(|&(_, c)| c == '=').unwrap_or(false) {
                                self.characters.next();
                                Ok((start_index, Token::EqualsLesser, start_index + 2))
                             } else if self.characters.peek().map(|&(_, c)| c == '-').unwrap_or(false) {
                                self.characters.next();
                                Ok((start_index, Token::LArrow, start_index + 2))
                            } else {
                                Ok((start_index, Token::Lesser, end_index))
                            }
                        }
                         '!' => {
                            if self.characters.peek().map(|&(_, c)| c == '=').unwrap_or(false) {
                                self.characters.next();
                                Ok((start_index, Token::BangEq, start_index + 2))
                            } else {
                                Ok((start_index, Token::Bang, end_index))
                            }
                        }

                        '"' => self.consume_string(start_index),
                        '\'' => self.consume_char(start_index),

                        c if c.is_ascii_digit() => self.consume_number(start_index, c),

                        c if c.is_ascii_alphabetic() || c == '_' => {
                             Ok(self.consume_identifier(start_index, c))
                        }
                         _ => Err(anyhow!("Unexpected character '{}' at index {}", current_char, start_index)),
                    };
                    return Some(result);
                }
            }
        }
    }
}