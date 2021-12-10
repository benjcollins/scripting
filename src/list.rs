use std::ops::{Index, IndexMut};

use crate::{heap::{HeapSlice, Heap}, vm::{Value, RustValue}};

#[derive(Debug)]
pub struct List<'func, 'src> {
    slice: HeapSlice<Value<'func, 'src>>,
}

impl<'func, 'src> List<'func, 'src> {
    pub fn new(heap: &mut Heap, length: usize) -> List<'func, 'src> {
        List { slice: heap.alloc_slice(length) }
    }
}

impl<'func, 'src> RustValue for List<'func, 'src> {
    fn call(&mut self) {
        todo!()
    }
}

impl<'func, 'src> Index<usize> for List<'func, 'src> {
    type Output = Value<'func, 'src>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.slice[index]
    }
}

impl<'func, 'src> IndexMut<usize> for List<'func, 'src> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.slice[index]
    }
}