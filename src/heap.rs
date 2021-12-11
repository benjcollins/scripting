use std::{alloc::{Layout, alloc, dealloc}, fmt, marker::PhantomData, mem::size_of, ops::{Deref, DerefMut, Index, IndexMut}, ptr::NonNull, slice};

pub struct Heap {
    base: *mut u8,
    offset: usize,
}

pub struct HeapPtr<T: ?Sized> {
    ptr: NonNull<T>,
    phantom: PhantomData<T>,
}

pub struct HeapSlice<T> {
    ptr: *mut T,
    length: usize,
}

impl Heap {
    pub fn new() -> Heap {
        unsafe {
            Heap {
                base: alloc(Layout::from_size_align(10000, 8).unwrap()),
                offset: 0
            }
        }
    }
    pub fn alloc_raw(&mut self, size: usize) -> *mut u8 {
        let ptr = unsafe { self.base.add(self.offset) };
        self.offset += size;
        ptr
    }
    pub fn alloc<T>(&mut self, data: T) -> HeapPtr<T> {
        let ptr = self.alloc_raw(size_of::<T>()) as *mut T;
        unsafe { *ptr = data };
        HeapPtr { ptr: NonNull::new(ptr).unwrap(), phantom: PhantomData }
    }
    pub fn alloc_slice<T>(&mut self, length: usize) -> HeapSlice<T> {
        let ptr = self.alloc_raw(size_of::<T>() * length) as *mut T;
        HeapSlice { ptr, length }
    }
}

impl<T: ?Sized> Clone for HeapPtr<T> {
    fn clone(&self) -> Self {
        Self { ptr: self.ptr, phantom: PhantomData }
    }
}

impl<T: ?Sized> Copy for HeapPtr<T> {}

impl<T> Clone for HeapSlice<T> {
    fn clone(&self) -> Self {
        Self { ptr: self.ptr, length: self.length }
    }
}

impl<T> Copy for HeapSlice<T> {}

impl<T: ?Sized> Deref for HeapPtr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T: ?Sized> DerefMut for HeapPtr<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.ptr.as_mut() }
    }
}

impl<T> Index<usize> for HeapSlice<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        if index >= self.length { panic!() }
        unsafe { &*self.ptr.add(index) }
    }
}

impl<T> IndexMut<usize> for HeapSlice<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index >= self.length { panic!() }
        unsafe { &mut *self.ptr.add(index) }
    }
}

impl Drop for Heap {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.base, Layout::from_size_align_unchecked(10000, 8));
        }
    }
}

impl<T> HeapSlice<T> {
    pub fn len(&self) -> usize {
        self.length
    }
    pub fn iter(&self) -> HeapSliceIter<T> {
        HeapSliceIter { slice: self, index: 0 }
    }
    // pub fn iter_mut(&self) -> HeapSliceIterMut<T> {

    // }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for HeapPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HeapPtr({:?})", unsafe { self.ptr.as_ref() })
    }
}

impl<T: fmt::Debug> fmt::Debug for HeapSlice<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HeapSlice[")?;
        if self.length > 0 {
            write!(f, "{:?}", self[0])?;
            for i in 1..self.length {
                write!(f, ", {:?}", self[i])?;
            }
        }
        write!(f, "]")
    }
}

pub struct HeapSliceIter<'slice, T> {
    slice: &'slice HeapSlice<T>,
    index: usize,
}

impl<'slice, T> Iterator for HeapSliceIter<'slice, T> {
    type Item = &'slice T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.slice.len() {
            self.index += 1;
            Some(&self.slice[self.index - 1])
        } else {
            None
        }
    }
}

pub struct HeapSliceIterMut<'slice, T> {
    slice: &'slice mut HeapSlice<T>,
    index: usize,
}

// impl<'slice, T> Iterator for HeapSliceIterMut<'slice, T> {
//     type Item = &'slice mut T;

//     fn next(&mut self) -> Option<Self::Item> {
//         if self.index < self.slice.len() {
//             self.index += 1;
//             Some(&mut self.slice[self.index - 1])
//         } else {
//             None
//         }
//     }
// }