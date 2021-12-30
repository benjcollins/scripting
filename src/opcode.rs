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
    PushFloat,
    PushTrue,
    PushFalse,
    PushNone,
    PushFunc,
    PushLoad,
    PushClosureLoad,
    PushList,
    PushPropLoad,

    PopStore,
    PopPrint,
    PopPropStore,
    PopClosureStore,

    Jump,
    JumpIfNot,
    Drop,

    Call,
    Return,

    Finish,
}