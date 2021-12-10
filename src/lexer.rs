use crate::token::{Position, Token, TokenType};

pub struct Lexer<'src> {
    source: &'src str,
    pos: Position,
    offset: usize,
}

impl<'src> Lexer<'src> {
    pub fn new(source: &'src str) -> Lexer {
        Lexer { source, pos: Position { line: 1, column: 1 }, offset: 0 }
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
        if ch == '\n' {
            self.pos.line += 1;
            self.pos.column = 1;
        } else {
            self.pos.column += 1;
        }
    }
    pub fn is_end(&self) -> bool {
        self.offset >= self.source.len()
    }
    fn new_token(&self, offset: usize, pos: Position, ty: TokenType) -> Token<'src> {
        Token { pos, source: &self.source[offset..self.offset], ty }
    }
    fn single_char_token(&mut self, ty: TokenType) -> Token<'src> {
        let pos = self.pos;
        let offset = self.offset;
        self.next_char();
        self.new_token(offset, pos, ty)
    }
    fn double_char_token_if(&mut self, ch: char, single: TokenType, double: TokenType) -> Token<'src> {
        let pos = self.pos;
        let offset = self.offset;
        self.next_char();
        if self.peek_char().map_or(false, |ch1| ch == ch1) {
            self.next();
            self.new_token(offset, pos, double)
        } else {
            self.new_token(offset, pos, single)
        }
    }
    pub fn next_token(&mut self) -> Token<'src> {
        loop {
            let offset = self.offset;
            let pos = self.pos;

            let ch = match self.peek_char() {
                Some(ch) => ch,
                None => return self.new_token(offset, pos, TokenType::End),
            };

            match ch {
                ch if ch.is_alphabetic() => {
                    while self.peek_char().map_or(false, char::is_alphanumeric) {
                        self.next_char();
                    }
                    let source = &self.source[offset..self.offset];
                    let ty = match source {
                        "true" => TokenType::True,
                        "false" => TokenType::False,
                        "none" => TokenType::None,

                        "var" => TokenType::Var,
                        "while" => TokenType::While,
                        "if" => TokenType::If,
                        "else" => TokenType::Else,
                        "func" => TokenType::Func,
                        "return" => TokenType::Return,

                        "print" => TokenType::Print,
                        _ => TokenType::Ident,
                    };
                    return self.new_token(offset, pos, ty);
                }
                ch if ch.is_numeric() => {
                    while self.peek_char().map_or(false, char::is_numeric) {
                        self.next_char();
                    }
                    return self.new_token(offset, pos, TokenType::Int)
                }
                ch if ch.is_whitespace() => {
                    while self.peek_char().map_or(false, char::is_whitespace) {
                        self.next_char();
                    }
                    continue
                }

                '"' => {
                    self.next_char();
                    while self.peek_char().map_or(false, |ch| ch != '"') {
                        self.next_char();
                    }
                    if self.peek_char() == None {
                        return self.new_token(offset, pos, TokenType::Invalid)
                    } else {
                        return self.new_token(offset, pos, TokenType::String)
                    }
                }

                '(' => return self.single_char_token(TokenType::OpenBrace),
                ')' => return self.single_char_token(TokenType::CloseBrace),
                '{' => return self.single_char_token(TokenType::OpenCurlyBrace),
                '}' => return self.single_char_token(TokenType::CloseCurlyBrace),
                '[' => return self.single_char_token(TokenType::OpenSquareBrace),
                ']' => return self.single_char_token(TokenType::CloseSquareBrace),
                ';' => return self.single_char_token(TokenType::SemiColon),
                ',' => return self.single_char_token(TokenType::Comma),

                '+' => return self.double_char_token_if('=', TokenType::Plus, TokenType::PlusEquals),
                '-' => return self.double_char_token_if('=', TokenType::Minus, TokenType::MinusEquals),
                '*' => return self.double_char_token_if('=', TokenType::Multiply, TokenType::MultiplyEquals),
                '/' => return self.double_char_token_if('=', TokenType::Divide, TokenType::DivideEquals),
                '%' => return self.double_char_token_if('=', TokenType::Modulus, TokenType::ModulusEquals),

                '!' => return self.double_char_token_if('=', TokenType::Not, TokenType::NotEqual),
                '=' => return self.double_char_token_if('=', TokenType::Equals, TokenType::DoubleEquals),
                '<' => return self.double_char_token_if('=', TokenType::Less, TokenType::LessOrEqual),
                '>' => return self.double_char_token_if('=', TokenType::Greater, TokenType::GreaterOrEqual),
                '&' => return self.double_char_token_if('&', TokenType::Invalid, TokenType::And),
                '|' => return self.double_char_token_if('|', TokenType::Invalid, TokenType::Or),
                
                _ => {
                    self.next_char();
                    return self.new_token(offset, pos, TokenType::Invalid)
                },
            }
        }
    }
}

impl<'src> Iterator for Lexer<'src> {
    type Item = Token<'src>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_token())
    }
}