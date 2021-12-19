use std::fmt;

use crate::{heap::{HeapSlice, Heap}, vm::{Value, RustValue, VirtualMachine}};

#[derive(Debug, Clone)]
pub struct List<'a> {
    slice: HeapSlice<Value<'a>>,
}

impl<'a> List<'a> {
    pub fn new(heap: &mut Heap, length: usize, stack: &mut Vec<Value<'a>>) -> List<'a> {
        let slice = heap.alloc_slice(length);
        for item in slice.iter_mut().rev() {
            *item = stack.pop().unwrap();
        }
        List { slice }
    }
}

impl<'a> RustValue<'a> for List<'a> {
    fn get_property(&mut self, index: u8, vm: &mut VirtualMachine<'a>) -> Value<'a> {
        match vm.props[index as usize] {
            "len" => Value::Int(self.slice.len() as i64),
            _ => panic!()
        }
    }
}

impl<'a> fmt::Display for List<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        let mut iter = self.slice.iter();
        if self.slice.len() > 0 {
            write!(f, "{}", iter.next().unwrap())?;
            for item in iter {
                write!(f, ", {}", item)?;
            }
        }
        write!(f, "]")
    }
}