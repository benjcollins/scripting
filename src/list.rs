use std::fmt;

use crate::{heap::{HeapSlice, Heap}, vm::{Value, RustValue, VirtualMachine}};

#[derive(Debug, Clone)]
pub struct List<'func, 'src> {
    slice: HeapSlice<Value<'func, 'src>>,
}

impl<'func, 'src> List<'func, 'src> {
    pub fn new(heap: &mut Heap, length: usize, stack: &mut Vec<Value<'func, 'src>>) -> List<'func, 'src> {
        let slice = heap.alloc_slice(length);
        for item in slice.iter_mut().rev() {
            *item = stack.pop().unwrap();
        }
        List { slice }
    }
}

impl<'func, 'src> RustValue<'func, 'src> for List<'func, 'src> {
    fn get_property(&mut self, index: u8, vm: &mut VirtualMachine<'func, 'src>) -> Value<'func, 'src> {
        match vm.props[index as usize] {
            "len" => Value::Int(self.slice.len() as i64),
            _ => panic!()
        }
    }
}

impl<'func, 'src> fmt::Display for List<'func, 'src> {
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