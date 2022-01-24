use std::fmt;

use crate::{heap::HeapPtr, parser::Program, vm::VirtualMachine};

#[derive(Debug, Clone, Copy)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Closure(HeapPtr<Closure>),
    RustValue(HeapPtr<dyn RustValue>),
    None,
}

pub struct DispValue<'a> {
    program: &'a Program,
    value: Value,
}

pub trait RustValue where Self: fmt::Debug + fmt::Display {
    fn get_property(&mut self, index: u8, vm: &mut VirtualMachine) -> Value;
}

#[derive(Debug, Clone)]
pub struct Closure {
    pub func_id: usize,
    pub closure_values: Vec<HeapPtr<ClosureValueRef>>,
}

#[derive(Debug, Clone, Copy)]
pub enum ClosureValueRef {
    Stack(usize),
    Heap(HeapPtr<Value>),
}

impl<'a> DispValue<'a> {
    pub fn new(value: Value, program: &'a Program) -> DispValue<'a> {
        DispValue { program, value }
    }
}

impl<'a> fmt::Display for DispValue<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.value {
            Value::Int(int) => write!(f, "{}", int),
            Value::Float(float) => write!(f, "{}", float),
            Value::Bool(bool) => write!(f, "{}", bool),
            Value::None => write!(f, "none"),
            Value::Closure(closure) => {
                let params: Vec<_> = self.program.funcs[closure.func_id].param_names.iter().map(|symbol| self.program.symbols.get_name(*symbol)).collect();
                write!(f, "func({})", params.join(", "))
            },
            Value::RustValue(value) => write!(f, "{}", &*value),
        }
    }
}