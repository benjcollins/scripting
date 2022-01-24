use std::fmt;

use crate::{heap::{HeapSlice, Heap}, vm::{Value, RustValue, VirtualMachine}, symbols::Symbol};

#[derive(Debug, Clone)]
pub struct List {
    slice: HeapSlice<Value>,
}

impl List {
    pub fn new(heap: &mut Heap, length: usize, stack: &mut Vec<Value>) -> List {
        let slice = heap.alloc_slice(length);
        for item in slice.iter_mut().rev() {
            *item = stack.pop().unwrap();
        }
        List { slice }
    }
}

impl RustValue for List {
    fn get_property(&mut self, index: u8, vm: &mut VirtualMachine) -> Value {
        match vm.program.symbols.get_name(Symbol::from_index(index as u32)) {
            "len" => Value::Int(self.slice.len() as i64),
            _ => panic!()
        }
    }
}

impl fmt::Display for List {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        let mut iter = self.slice.iter();
        if self.slice.len() > 0 {
            write!(f, "{:?}", iter.next().unwrap())?;
            for item in iter {
                write!(f, ", {:?}", item)?;
            }
        }
        write!(f, "]")
    }
}