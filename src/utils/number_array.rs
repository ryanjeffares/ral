use std::{
    alloc::{alloc, alloc_zeroed, dealloc, handle_alloc_error, Layout},
    borrow::{Borrow, BorrowMut},
    fmt,
    ops::{Deref, DerefMut, Index, IndexMut}, marker::PhantomData,
};

macro_rules! impl_trait_for_types {
    ($trait_name:ident, $($type_name:ident),+) => {
        $(impl $trait_name for $type_name {
            const SIZE: usize = std::mem::size_of::<$type_name>();
            const ALIGNMENT: usize = std::mem::align_of::<$type_name>();
        })*
    };
}

pub trait Number: Copy {
    const SIZE: usize;
    const ALIGNMENT: usize;
}

impl_trait_for_types!(
    Number, i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, f32, f64, usize, isize
);

pub struct NumberArray<T>
where
    T: Number,
{
    ptr: *mut T,
    len: usize,
    phantom: PhantomData<T>,
}

pub struct IntoIter<T>
where
    T: Number,
{
    array: NumberArray<T>,
    index: usize,
}

impl<T: Number> NumberArray<T> {
    pub fn new(len: usize) -> Self {
        let ptr = unsafe {
            let layout = Layout::from_size_align_unchecked(len * T::SIZE, T::ALIGNMENT);
            let p = alloc_zeroed(layout);
            if p.is_null() {
                handle_alloc_error(layout);
            }
            p as *mut T
        };

        Self { ptr, len, phantom: PhantomData }
    }

    pub fn new_uninitialised(len: usize) -> Self {
        let ptr = unsafe {
            let layout = Layout::from_size_align_unchecked(len * T::SIZE, T::ALIGNMENT);
            let p = alloc(layout);
            if p.is_null() {
                handle_alloc_error(layout);
            }
            p as *mut T
        };

        Self { ptr, len, phantom: PhantomData }
    }

    pub fn new_with_value(len: usize, value: T) -> Self {
        let ptr = unsafe {
            let layout = Layout::from_size_align_unchecked(len * T::SIZE, T::ALIGNMENT);
            let p = alloc(layout) as *mut T;
            if p.is_null() {
                handle_alloc_error(layout);
            }
            for i in 0..len {
                *p.add(i) = value;
            }
            p
        };

        Self { ptr, len, phantom: PhantomData }
    }

    pub fn fill(&mut self, value: T) {
        unsafe {
            for i in 0..self.len {
                *(self.ptr.add(i)) = value;
            }
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if index < self.len {
            unsafe { Some(&*(self.ptr.add(index))) }
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index < self.len {
            unsafe { Some(&mut *(self.ptr.add(index))) }
        } else {
            None
        }
    }
}

impl<T: Number> Drop for NumberArray<T> {
    fn drop(&mut self) {
        unsafe {
            dealloc(
                self.ptr as *mut u8,
                Layout::from_size_align_unchecked(self.len * T::SIZE, T::ALIGNMENT),
            )
        }
    }
}

impl<T: Number> Index<usize> for NumberArray<T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).unwrap()
    }
}

impl<T: Number> IndexMut<usize> for NumberArray<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index).unwrap()
    }
}

impl<'a, T: Number> IntoIterator for &'a NumberArray<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T: Number> IntoIterator for &'a mut NumberArray<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<T: Number> IntoIterator for NumberArray<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            array: self,
            index: 0,
        }
    }
}

impl<T: Number> Iterator for IntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.array.len() {
            None
        } else {
            unsafe {
                let result = Some(*self.array.ptr.add(self.index));
                self.index += 1;
                result
            }
        }
    }
}

// impl<'a, T: Number> Into<&'a [T]> for NumberArray<T> {
//     fn into(self) -> &'a [T] {
//         unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
//     }
// }

impl<T: Number> From<&NumberArray<T>> for &[T] {
    fn from(value: &NumberArray<T>) -> Self {
        unsafe { std::slice::from_raw_parts(value.ptr, value.len) }
    }
}

impl<T: Number> From<&[T]> for NumberArray<T> {
    fn from(slice: &[T]) -> Self {
        let mut array = NumberArray::<T>::new(slice.len());
        for i in 0..slice.len() {
            array[i] = slice[i];
        }
        array
    }
}

impl<T: Number> From<&mut [T]> for NumberArray<T> {
    fn from(slice: &mut [T]) -> Self {
        let mut array = NumberArray::<T>::new(slice.len());
        for i in 0..slice.len() {
            array[i] = slice[i];
        }
        array
    }
}

impl<T: Number> Deref for NumberArray<T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl<T: Number> DerefMut for NumberArray<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl<T: Number> AsMut<[T]> for NumberArray<T> {
    fn as_mut(&mut self) -> &mut [T] {
        self
    }
}

impl<T: Number> AsMut<NumberArray<T>> for NumberArray<T> {
    fn as_mut(&mut self) -> &mut NumberArray<T> {
        self
    }
}

impl<T: Number> AsRef<[T]> for NumberArray<T> {
    fn as_ref(&self) -> &[T] {
        self
    }
}

impl<T: Number> AsRef<NumberArray<T>> for NumberArray<T> {
    fn as_ref(&self) -> &NumberArray<T> {
        self
    }
}

impl<T: Number> Borrow<[T]> for NumberArray<T> {
    fn borrow(&self) -> &[T] {
        self
    }
}

impl<T: Number> BorrowMut<[T]> for NumberArray<T> {
    fn borrow_mut(&mut self) -> &mut [T] {
        self
    }
}

impl<T: Number + fmt::Debug> fmt::Debug for NumberArray<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: Number> Clone for NumberArray<T> {
    fn clone(&self) -> Self {
        unsafe {
            let ptr = alloc(Layout::from_size_align_unchecked(
                self.len * T::SIZE,
                T::ALIGNMENT,
            )) as *mut T;
            for i in 0..self.len {
                *ptr.add(i) = self[i];
            }
            NumberArray { ptr, len: self.len, phantom: PhantomData }
        }
    }
}
