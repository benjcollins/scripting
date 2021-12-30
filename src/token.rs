#[derive(Debug, Clone, Copy)]
pub struct Token<'src> {
    pub pos: Position,
    pub kind: TokenKind<'src>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenKind<'src> {
    Ident(&'src str),
    Int(u64),
    Float(f32),
    String(&'src str),
    
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
    Dot,
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

    List,

    End,
    Invalid,
}

#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub line: u32,
    pub column: u32,
}