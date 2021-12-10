#[derive(Debug, Clone, Copy)]
pub struct Token<'src> {
    pub source: &'src str,
    pub pos: Position,
    pub ty: TokenType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenType {
    Ident,
    Int,
    String,
    
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulus,

    NotEqual,
    DoubleEquals,
    Less,
    Greater,
    LessOrEqual,
    GreaterOrEqual,

    Not,
    And,
    Or,

    PlusEquals,
    MinusEquals,
    MultiplyEquals,
    DivideEquals,
    ModulusEquals,

    OpenBrace,
    CloseBrace,
    OpenCurlyBrace,
    CloseCurlyBrace,
    OpenSquareBrace,
    CloseSquareBrace,
    SemiColon,
    Comma,
    Equals,

    Var,
    True,
    False,
    None,
    While,
    If,
    Else,
    Func,
    Return,
    Print,

    End,
    Invalid,
}

#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub line: u32,
    pub column: u32,
}