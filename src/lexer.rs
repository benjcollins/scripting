use crate::token::{Token, TokenKind};

pub struct Lexer<'src> {
    source: &'src str,
    offset: usize,
}

impl<'src> Lexer<'src> {
    pub fn new(source: &'src str) -> Lexer {
        Lexer { source, offset: 0 }
    }
    fn peek_char(&self) -> Option<char> {
        self.source[self.offset..].chars().next()
    }
    fn next_char(&mut self) {
        let mut chars = self.source[self.offset..].chars();
        let ch = match chars.next() {
            Some(ch) => ch,
            None => return
        };
        self.offset += ch.len_utf8();
    }
    fn single_char_token(&mut self, token: TokenKind<'src>) -> TokenKind<'src> {
        self.next_char();
        token
    }
    fn double_char_token_if(&mut self, ch: char, single: TokenKind<'src>, double: TokenKind<'src>) -> TokenKind<'src> {
        self.next_char();
        if self.peek_char().map_or(false, |ch1| ch == ch1) {
            self.next_token();
            double
        } else {
            single
        }
    }
    pub fn next_token(&mut self) -> Token<'src> {
        let mut offset;
        let kind = loop {
            offset = self.offset;

            let ch = match self.peek_char() {
                Some(ch) => ch,
                None => break TokenKind::End,
            };

            match ch {
                ch if ch.is_alphabetic() => {
                    let start = self.offset;
                    while self.peek_char().map_or(false, char::is_alphanumeric) {
                        self.next_char();
                    }
                    break match &self.source[start..self.offset] {
                        "true" => TokenKind::True,
                        "false" => TokenKind::False,
                        "none" => TokenKind::None,

                        "var" => TokenKind::Var,
                        "while" => TokenKind::While,
                        "if" => TokenKind::If,
                        "else" => TokenKind::Else,
                        "func" => TokenKind::Func,
                        "return" => TokenKind::Return,

                        "list" => TokenKind::List,

                        "print" => TokenKind::Print,

                        name => TokenKind::Ident(name),
                    }
                }
                ch if ch.is_numeric() => {
                    let start = self.offset;
                    while self.peek_char().map_or(false, char::is_numeric) {
                        self.next_char();
                    }
                    break if self.peek_char() == Some('.') {
                        self.next_char();
                        while self.peek_char().map_or(false, char::is_numeric) {
                            self.next_char()
                        }
                        TokenKind::Float(self.source[start..self.offset].parse().unwrap())
                    } else {
                        TokenKind::Int(self.source[start..self.offset].parse().unwrap())
                    }
                }
                ch if ch.is_whitespace() => {
                    while self.peek_char().map_or(false, char::is_whitespace) {
                        self.next_char();
                    }
                }
                '"' => {
                    self.next_char();
                    let start = self.offset;
                    while self.peek_char().map_or(false, |ch| ch != '"') {
                        self.next_char();
                    }
                    break match self.peek_char() {
                        Some(_) => {
                            let str = &self.source[start..self.offset];
                            self.next_char();
                            TokenKind::String(str)
                        }
                        None => TokenKind::Invalid
                    }
                }

                '(' => break self.single_char_token(TokenKind::OpenBrace),
                ')' => break self.single_char_token(TokenKind::CloseBrace),
                '{' => break self.single_char_token(TokenKind::OpenCurlyBrace),
                '}' => break self.single_char_token(TokenKind::CloseCurlyBrace),
                '[' => break self.single_char_token(TokenKind::OpenSquareBrace),
                ']' => break self.single_char_token(TokenKind::CloseSquareBrace),
                ';' => break self.single_char_token(TokenKind::SemiColon),
                ',' => break self.single_char_token(TokenKind::Comma),
                '.' => break self.single_char_token(TokenKind::Dot),

                '+' => break self.double_char_token_if('=', TokenKind::Plus, TokenKind::PlusEquals),
                '-' => break self.double_char_token_if('=', TokenKind::Minus, TokenKind::MinusEquals),
                '*' => break self.double_char_token_if('=', TokenKind::Multiply, TokenKind::MultiplyEquals),
                '/' => break self.double_char_token_if('=', TokenKind::Divide, TokenKind::DivideEquals),
                '%' => break self.double_char_token_if('=', TokenKind::Modulus, TokenKind::ModulusEquals),

                '!' => break self.double_char_token_if('=', TokenKind::Not, TokenKind::NotEqual),
                '=' => break self.double_char_token_if('=', TokenKind::Equals, TokenKind::DoubleEquals),
                '<' => break self.double_char_token_if('=', TokenKind::Less, TokenKind::LessOrEqual),
                '>' => break self.double_char_token_if('=', TokenKind::Greater, TokenKind::GreaterOrEqual),
                '&' => break self.double_char_token_if('&', TokenKind::Invalid, TokenKind::And),
                '|' => break self.double_char_token_if('|', TokenKind::Invalid, TokenKind::Or),
                
                _ => {
                    self.next_char();
                    break TokenKind::Invalid
                },
            }
        };
        Token { kind, offset }
    }
}