use std::{
    alloc::{self, Layout},
    cell::Cell,
    marker::PhantomData,
    mem,
    ops::{Deref, DerefMut},
    ptr::{self, NonNull},
    rc::Rc,
};

#[derive(Debug)]
struct ArenaElement {
    value: NonNull<u8>,
    drop: unsafe fn(NonNull<u8>),
    layout: Layout,
}

impl Drop for ArenaElement {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe {
            (self.drop)(self.value);
        }
    }
}

#[derive(Debug)]
pub struct Arena {
    memory: NonNull<u8>,
    size: usize,
    offset: usize,
    elements: Vec<ArenaElement>,
    valid: Rc<Cell<bool>>,
}

impl Arena {
    pub fn new(size_in_bytes: usize) -> Result<Self, alloc::AllocError> {
        if size_in_bytes == 0 {
            return Err(alloc::AllocError);
        }

        unsafe {
            let layout = Layout::from_size_align(size_in_bytes, mem::align_of::<usize>())
                .map_err(|_| alloc::AllocError)?;
            let memory = NonNull::new(alloc::alloc(layout))
                .ok_or(alloc::AllocError)?;

            Ok(Self {
                memory,
                size: size_in_bytes,
                offset: 0,
                elements: Vec::new(),
                valid: Rc::new(Cell::new(true)),
            })
        }
    }

    pub fn len(&self) -> usize {
        self.offset
    }

    pub fn capacity(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.offset == 0
    }

    pub fn clear(&mut self) {
        self.valid.set(false);
        self.valid = Rc::new(Cell::new(true));
        self.elements.clear();
        self.offset = 0;
    }

    #[inline]
    pub fn alloc<T>(&mut self, f: impl FnOnce() -> T) -> Result<ArenaBox<T>, alloc::AllocError> {
        let layout = Layout::new::<T>();
        let padding = self.padding_needed_for(layout.align());
        let size = layout.size();

        if self.offset + padding + size > self.size {
            return Err(alloc::AllocError);
        }

        unsafe {
            let aligned_offset = self.offset + padding;
            let ptr = NonNull::new_unchecked(self.memory.as_ptr().add(aligned_offset));

            // Write the value first
            ptr::write(ptr.as_ptr() as *mut T, f());

            // Create result after successful write
            let result = ArenaBox {
                ptr: ptr.cast(),
                valid: self.valid.clone(),
                _marker: PhantomData,
            };

            // Record the element for cleanup
            self.elements.push(ArenaElement {
                value: ptr,
                drop: Self::drop_element::<T>,
                layout,
            });

            self.offset = aligned_offset + size;
            Ok(result)
        }
    }

    #[inline]
    fn padding_needed_for(&self, align: usize) -> usize {
        let offset = self.offset;
        let mask = align - 1;
        let misalignment = offset & mask;
        if misalignment > 0 {
            align - misalignment
        } else {
            0
        }
    }

    unsafe fn drop_element<T>(ptr: NonNull<u8>) {
        ptr::drop_in_place(ptr.as_ptr() as *mut T);
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        self.clear();
        unsafe {
            let layout = Layout::from_size_align_unchecked(self.size, mem::align_of::<usize>());
            alloc::dealloc(self.memory.as_ptr(), layout);
        }
    }
}

#[derive(Debug)]
pub struct ArenaBox<T: ?Sized> {
    ptr: NonNull<T>,
    valid: Rc<Cell<bool>>,
    _marker: PhantomData<T>,
}

impl<T: ?Sized> ArenaBox<T> {
    #[inline]
    pub fn map<U: ?Sized>(mut self, f: impl FnOnce(&mut T) -> &mut U) -> ArenaBox<U> {
        let mapped_ptr = NonNull::from(f(unsafe { self.ptr.as_mut() }));
        ArenaBox {
            ptr: mapped_ptr,
            valid: self.valid.clone(),
            _marker: PhantomData,
        }
    }

    #[track_caller]
    fn validate(&self) {
        assert!(
            self.valid.get(),
            "attempted to dereference an ArenaRef after its Arena was cleared"
        );
    }

    pub fn as_ptr(&self) -> *mut T {
        self.ptr.as_ptr()
    }
}

impl<T: ?Sized> Deref for ArenaBox<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.validate();
        unsafe { self.ptr.as_ref() }
    }
}

impl<T: ?Sized> DerefMut for ArenaBox<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.validate();
        unsafe { self.ptr.as_mut() }
    }
}

pub struct ArenaRef<T: ?Sized>(ArenaBox<T>);

impl<T: ?Sized> From<ArenaBox<T>> for ArenaRef<T> {
    fn from(value: ArenaBox<T>) -> Self {
        ArenaRef(value)
    }
}

impl<T: ?Sized> Clone for ArenaRef<T> {
    fn clone(&self) -> Self {
        Self(ArenaBox {
            ptr: self.0.ptr,
            valid: self.0.valid.clone(),
            _marker: PhantomData,
        })
    }
}

impl<T: ?Sized> Deref for ArenaRef<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

#[cfg(test)]
mod tests {
    use std::{alloc, cell::Cell, rc::Rc};

    use super::*;

    #[test]
    fn test_arena() {
        let mut arena = Arena::new(1024).unwrap();
        let a = arena.alloc(|| 1u64).unwrap();
        let b = arena.alloc(|| 2u32).unwrap();
        let c = arena.alloc(|| 3u16).unwrap();
        let d = arena.alloc(|| 4u8).unwrap();
        assert_eq!(*a, 1);
        assert_eq!(*b, 2);
        assert_eq!(*c, 3);
        assert_eq!(*d, 4);

        arena.clear();
        let a = arena.alloc(|| 5u64).unwrap();
        let b = arena.alloc(|| 6u32).unwrap();
        let c = arena.alloc(|| 7u16).unwrap();
        let d = arena.alloc(|| 8u8).unwrap();
        assert_eq!(*a, 5);
        assert_eq!(*b, 6);
        assert_eq!(*c, 7);
        assert_eq!(*d, 8);

        // Ensure drop gets called.
        let dropped = Rc::new(Cell::new(false));
        struct DropGuard(Rc<Cell<bool>>);
        impl Drop for DropGuard {
            fn drop(&mut self) {
                self.0.set(true);
            }
        }
        arena.alloc(|| DropGuard(dropped.clone())).unwrap();
        arena.clear();
        assert!(dropped.get());
    }

    #[test]
    fn test_arena_overflow() {
        let mut arena = Arena::new(16).unwrap();
        arena.alloc(|| 1u64).unwrap();
        arena.alloc(|| 2u64).unwrap();
        // This should return an error
        assert!(arena.alloc(|| 3u64).is_err());
    }

    #[test]
    fn test_zero_size_arena() {
        assert!(Arena::new(0).is_err());
    }

    #[test]
    fn test_arena_alignment() {
        let mut arena = Arena::new(256).unwrap();
        let x1 = arena.alloc(|| 1u8).unwrap();
        let x2 = arena.alloc(|| 2u16).unwrap();
        let x3 = arena.alloc(|| 3u32).unwrap();
        let x4 = arena.alloc(|| 4u64).unwrap();
        let x5 = arena.alloc(|| 5u64).unwrap();

        assert_eq!(*x1, 1);
        assert_eq!(*x2, 2);
        assert_eq!(*x3, 3);
        assert_eq!(*x4, 4);
        assert_eq!(*x5, 5);

        // Verify alignment using NonNull's as_ptr()
        let x1_ptr = x1.as_ptr();
        let x2_ptr = x2.as_ptr();
        assert_eq!(x1_ptr as usize % mem::align_of::<u8>(), 0);
        assert_eq!(x2_ptr as usize % mem::align_of::<u16>(), 0);
    }

    #[test]
    #[should_panic(expected = "attempted to dereference an ArenaRef after its Arena was cleared")]
    fn test_arena_use_after_clear() {
        let mut arena = Arena::new(16).unwrap();
        let value = arena.alloc(|| 1u64).unwrap();

        arena.clear();
        let _read_value = *value;
    }
}
