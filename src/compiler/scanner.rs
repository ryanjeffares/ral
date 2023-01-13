use phf::phf_map;
use std::fmt;

use crate::runtime::instrument::VariableType;

static KEYWORDS: phf::Map<&'static str, TokenType> = phf_map! {
    "instruments" => TokenType::InstrumentsIdent,
    "score" => TokenType::ScoreIdent,
    "Int" => TokenType::IntIdent,
    "Float" => TokenType::FloatIdent,
    "Audio" => TokenType::AudioIdent,
    "String" => TokenType::StringIdent,
    "init" => TokenType::InitIdent,
    "perf" => TokenType::PerfIdent,
    "print" => TokenType::Print,
    "println" => TokenType::PrintLn,
    "local" => TokenType::Local,
};

static SYMBOLS: phf::Map<&'static str, TokenType> = phf_map! {
    "{" => TokenType::BraceOpen,
    "}" => TokenType::BraceClose,
    ":" => TokenType::Colon,
    "," => TokenType::Comma,
    "=" => TokenType::Equal,
    "(" => TokenType::ParenOpen,
    ")" => TokenType::ParenClose,
    ";" => TokenType::Semicolon,
};

pub struct Scanner {
    code: String,
    start: usize,
    current: usize,
    line: usize,
    column: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TokenType {
    AudioIdent,
    BraceOpen,
    BraceClose,
    Colon,
    Comma,
    Equal,
    ErrorToken,
    Float,
    FloatIdent,
    Identifier,
    InitIdent,
    InstrumentsIdent,
    IntIdent,
    Integer,
    Local,
    ParenOpen,
    ParenClose,
    PerfIdent,
    Print,
    PrintLn,
    ScoreIdent,
    Semicolon,
    String,
    StringIdent,
}

pub struct Token {
    text: String,
    start: usize,
    end: usize,
    line: usize,
    column: usize,
    token_type: TokenType,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Token: [ text: '{}', type: {:?}, start: {}, end: {}, line: {}, column: {} ]",
            self.text, self.token_type, self.start, self.end, self.line, self.column,
        )
    }
}

impl Token {
    pub fn token_type(&self) -> TokenType {
        self.token_type
    }

    pub fn text(&self) -> &String {
        &self.text
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn column(&self) -> usize {
        self.column
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }
}

impl Clone for Token {
    fn clone(&self) -> Self {
        Token {
            text: self.text.clone(),
            start: self.start,
            end: self.end,
            line: self.line,
            column: self.column,
            token_type: self.token_type,
        }
    }
}

impl TokenType {
    pub fn is_type_ident(&self) -> bool {
        *self == TokenType::FloatIdent
            || *self == TokenType::IntIdent
            || *self == TokenType::AudioIdent
            || *self == TokenType::StringIdent
    }

    pub fn to_variable_type(&self) -> VariableType {
        match *self {
            TokenType::IntIdent => VariableType::Int,
            TokenType::FloatIdent => VariableType::Float,
            TokenType::StringIdent => VariableType::String,
            TokenType::AudioIdent => VariableType::Audio,
            _ => panic!("Cannot convert {self:?} to VariableType"),
        }
    }

    pub fn is_literal(&self) -> bool {
        *self == TokenType::Integer || *self == TokenType::Float || *self == TokenType::String
    }

    pub fn is_operator(&self) -> bool {
        *self == TokenType::Equal
    }
}

impl Scanner {
    pub fn new(code: String) -> Self {
        Scanner {
            code,
            start: 0,
            current: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn get_code_at_line(&self, line: usize) -> String {
        let mut curr = 1usize;
        let mut str_index = 0usize;

        while curr < line {
            if str_index >= self.code.len() {
                return "".to_string();
            }

            str_index += 1;
            if self.code.as_bytes()[str_index - 1] == b'\n' {
                curr += 1;
            }
        }

        let mut slice = &self.code.as_str()[str_index..];
        slice = &slice[0..slice.find('\n').unwrap_or(slice.len())];
        slice.to_string()
    }

    pub fn scan_token(&mut self) -> Option<Token> {
        if self.peek().is_none() {
            None
        } else {
            self.skip_whitespace();
            self.start = self.current;

            let current = self.advance()?;
            if current.is_alphabetic() || current == '_' {
                return Some(self.identifier());
            }

            if current.is_ascii_digit() {
                return Some(self.number());
            }

            if let Some(token_type) = SYMBOLS.get(&self.code[self.start..self.current]) {
                Some(self.make_token(*token_type))
            } else if current == '"' {
                Some(self.string())
            } else {
                Some(Token {
                    text: String::default(),
                    start: 0,
                    end: 0,
                    token_type: TokenType::ErrorToken,
                    line: 0,
                    column: 0,
                })
            }
        }
    }

    fn advance(&mut self) -> Option<char> {
        if self.current >= self.code.len() {
            None
        } else {
            self.current += 1;
            self.column += 1;
            Some(self.code.as_bytes()[self.current - 1] as char)
        }
    }

    fn peek(&self) -> Option<char> {
        if self.current >= self.code.len() {
            None
        } else {
            Some(self.code.as_bytes()[self.current] as char)
        }
    }

    fn peek_next(&self) -> Option<char> {
        if self.current >= self.code.len() - 1 {
            None
        } else {
            Some(self.code.as_bytes()[self.current + 1] as char)
        }
    }

    fn peek_previous(&self) -> Option<char> {
        if self.current == 0 {
            None
        } else {
            Some(self.code.as_bytes()[self.current - 1] as char)
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            match c {
                '\t' | '\r' | ' ' => {
                    self.advance();
                }
                '\n' => {
                    self.line += 1;
                    self.column = 0;
                    self.advance();
                }
                _ => break,
            }
        }
    }

    fn identifier(&mut self) -> Token {
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                self.advance();
            } else {
                break;
            }
        }

        if let Some(token_type) = KEYWORDS.get(&self.code[self.start..self.current]) {
            self.make_token(*token_type)
        } else {
            self.make_token(TokenType::Identifier)
        }
    }

    fn number(&mut self) -> Token {
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }

        if self.peek().unwrap_or_default() == '.'
            && self.peek_next().unwrap_or_default().is_ascii_digit()
        {
            self.advance();
            while self.peek().is_some() && self.peek().unwrap().is_ascii_digit() {
                self.advance();
            }

            self.make_token(TokenType::Float)
        } else {
            self.make_token(TokenType::Integer)
        }
    }

    fn string(&mut self) -> Token {
        loop {
            if let Some(c) = self.peek() {
                if c == '"' && self.peek_previous().unwrap() != '\\' {
                    break;
                }
                if c == '\n' {
                    self.line += 1;
                    self.column = 0;
                }
                self.advance();
            } else {
                return self.error_token("Unterminated string");
            }
        }

        self.advance();
        self.make_token(TokenType::String)
    }

    fn make_token(&self, token_type: TokenType) -> Token {
        Token {
            text: self.code[self.start..self.current].to_string(),
            start: self.start,
            end: self.current,
            token_type,
            line: self.line,
            column: self.column - (self.current - self.start),
        }
    }

    fn error_token(&self, message: &'static str) -> Token {
        println!("{}, {}, {}", self.column, self.current, self.start);
        Token {
            text: message.to_owned(),
            start: self.start,
            end: self.current,
            token_type: TokenType::ErrorToken,
            line: self.line,
            column: self.column,
        }
    }
}
