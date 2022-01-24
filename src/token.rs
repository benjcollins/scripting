#[derive(Debug, Clone, Copy)]
pub struct Token<'src> {
    pub offset: usize,
    pub kind: TokenKind<'src>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenKind<'src> {
    Ident(&'src str),
    Int(u64),
    Float(f64),
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

pub struct Position {
    pub line: u32,
    pub column: u32,
}

pub fn pos_at_offset(source: &str, offset: usize) -> Position {
    let mut line = 1;
    let mut column = 1;
    for (i, ch) in source.char_indices() {
        if i == offset {
            return Position { line, column }
        }
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    unreachable!()
}