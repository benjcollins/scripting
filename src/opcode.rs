use std::{fmt, convert::TryInto, mem::size_of};

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

pub struct Bytecode<'bytecode> {
    bytecode: &'bytecode [u8],
}

struct BytecodePrinter<'bytecode> {
    bytecode: &'bytecode [u8],
    offset: usize,
}

impl<'bytecode> BytecodePrinter<'bytecode> {
    fn take_bytes(&mut self, n: usize) -> &[u8] {
        let bytes = &self.bytecode[self.offset..self.offset + n];
        self.offset += n;
        bytes
    }
}

impl<'bytecode> Bytecode<'bytecode> {
    pub fn new(bytecode: &'bytecode [u8]) -> Bytecode<'bytecode> {
        Bytecode { bytecode }
    }
}

impl<'bytecode> fmt::Display for Bytecode<'bytecode> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut printer = BytecodePrinter { bytecode: self.bytecode, offset: 0 };
        while printer.offset < printer.bytecode.len() {
            write!(f, "{:>5} : ", printer.offset)?;
            let opcode: Opcode = printer.take_bytes(1)[0].try_into().unwrap();
            write!(f, "{:?} ", opcode)?;
            match opcode {
                Opcode::Add | Opcode::Subtract | Opcode::Multiply | Opcode::Divide | Opcode::Modulus |
                Opcode::Equal | Opcode::NotEqual | Opcode::Less | Opcode::Greater | Opcode::LessOrEqual | Opcode::GreaterOrEqual |
                Opcode::PushTrue | Opcode::PushFalse | Opcode::PushNone | Opcode::PopPrint |
                Opcode::Return | Opcode::Finish => writeln!(f, ""),

                Opcode::PushInt => writeln!(f, "{}", i64::from_be_bytes(printer.take_bytes(size_of::<i64>()).try_into().unwrap())),
                Opcode::PushLoad | Opcode::PopStore |
                Opcode::PushClosureLoad | Opcode::PopClosureStore |
                Opcode::Drop | Opcode::Call => writeln!(f, "{}", printer.take_bytes(1)[0]),
                Opcode::Jump | Opcode::JumpIfNot | Opcode::PushList => writeln!(f, "{}", u32::from_be_bytes(printer.take_bytes(size_of::<u32>()).try_into().unwrap())),
                Opcode::PushFunc => writeln!(f, "func{}", u32::from_be_bytes(printer.take_bytes(size_of::<u32>()).try_into().unwrap())),
            }?;
        }
        Ok(())
    }
}