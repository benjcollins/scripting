use num_enum::{IntoPrimitive, TryFromPrimitive};

#[derive(Debug, Clone, Copy, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum Opcode {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulus,

    Equal,
    NotEqual,
    Less,
    Greater,
    LessOrEqual,
    GreaterOrEqual,

    PushInt,
    PushTrue,
    PushFalse,
    PushNone,
    PushFunc,
    PushLoad,
    PushClosureLoad,
    PushList,

    PopStore,
    PopPrint,
    PopClosureStore,

    Jump,
    JumpIfNot,
    Drop,

    Call,
    Return,

    Finish,
}