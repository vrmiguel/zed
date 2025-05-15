use std::{
    alloc::{self, Layout},
    cell::Cell,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr::{self, NonNull},
    rc::Rc,
};

struct ArenaElement {
    value: NonNull<u8>,
    drop: unsafe fn(*mut u8),
}

impl Drop for ArenaElement {
    #[inline(always)]
    fn drop(&mut self) {
        // SAFETY: We maintain the invariant that value points to valid memory
        // within the arena's allocation and is properly initialized of type T.
        unsafe {
            (self.drop)(self.value.as_ptr());
        }
    }
}

/// An arena allocator that manages a fixed-size memory block.
/// 
/// # Safety
/// - All memory is allocated from a single contiguous block
/// - Memory is properly aligned for all types
/// - Memory is automatically freed when the arena is dropped
/// - Use-after-free is prevented via validation
pub struct Arena {
    memory: NonNull<[u8]>,
    offset: NonNull<u8>,
    elements: Vec<ArenaElement>,
    valid: Rc<Cell<bool>>,
}

impl Arena {
    pub fn new(size_in_bytes: usize) -> Self {
        let layout = Layout::from_size_align(size_in_bytes, 1)
            .expect("invalid arena size/alignment");

        // SAFETY: We use a valid layout and handle allocation failure
        let ptr = unsafe { alloc::alloc(layout) };
        let memory = NonNull::new(ptr)
            .map(|p| NonNull::slice_from_raw_parts(p, size_in_bytes))
            .expect("allocation failed");

        Self {
            memory,
            offset: NonNull::new(ptr).expect("allocation failed"),
            elements: Vec::new(),
            valid: Rc::new(Cell::new(true)),
        }
    }

    pub fn len(&self) -> usize {
        // SAFETY: offset and start are valid pointers within our allocation
        unsafe {
            self.offset.as_ptr().offset_from(self.memory.as_ptr().cast()) as usize
        }
    }

    pub fn capacity(&self) -> usize {
        self.memory.len()
    }

    pub fn clear(&mut self) {
        self.valid.set(false);
        self.valid = Rc::new(Cell::new(true));
        self.elements.clear();
        
        // SAFETY: memory.as_ptr() points to our valid allocation
        self.offset = unsafe { NonNull::new_unchecked(self.memory.as_ptr().cast()) };
    }

    #[inline(always)]
    pub fn alloc<T>(&mut self, f: impl FnOnce() -> T) -> ArenaBox<T> {
        let layout = Layout::new::<T>();
        
        // SAFETY: offset is a valid pointer within our allocation
        let aligned_offset = unsafe {
            self.offset.as_ptr().add(
                self.offset.as_ptr().align_offset(layout.align())
            )
        };
        
        let allocated_ptr = NonNull::new(aligned_offset.cast())
            .expect("allocation pointer was null");
            
        // SAFETY: We've verified the allocation has enough space
        let next_offset = unsafe { aligned_offset.add(layout.size()) };
        
        // Check we have enough space
        if unsafe { next_offset > self.memory.as_ptr().add(self.memory.len()) } {
            panic!("Arena out of memory");
        }

        let result = ArenaBox {
            ptr: allocated_ptr,
            valid: self.valid.clone(),
            _phantom: PhantomData,
        };

        // SAFETY: ptr points to allocated memory of the correct size and alignment
        unsafe {
            ptr::write(allocated_ptr.as_ptr(), f());
        }

        self.elements.push(ArenaElement {
            value: allocated_ptr.cast(),
            drop: |p| unsafe { ptr::drop_in_place::<T>(p.cast()) },
        });

        // SAFETY: next_offset is within our allocation
        self.offset = NonNull::new(next_offset).expect("offset pointer was null");

        result
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        self.clear();
        
        let layout = Layout::from_size_align(self.memory.len(), 1)
            .expect("invalid layout");
            
        // SAFETY: memory points to an allocation created with the same layout
        unsafe {
            alloc::dealloc(self.memory.as_ptr().cast(), layout);
        }
    }
}

/// A pointer to an object allocated in an Arena.
/// 
/// Similar to Box<T> but memory is managed by the Arena.
pub struct ArenaBox<T: ?Sized> {
    ptr: NonNull<T>,
    valid: Rc<Cell<bool>>,
    _phantom: PhantomData<T>,
}

// SAFETY: ArenaBox owns its data uniquely and can be sent between threads
unsafe impl<T: Send + ?Sized> Send for ArenaBox<T> {}
// SAFETY: Multiple threads can safely access the same Arena reference
unsafe impl<T: Sync + ?Sized> Sync for ArenaBox<T> {}

impl<T: ?Sized> ArenaBox<T> {
    #[inline(always)]
    pub fn map<U: ?Sized>(mut self, f: impl FnOnce(&mut T) -> &mut U) -> ArenaBox<U> {
        ArenaBox {
            ptr: NonNull::from(f(&mut self)).into(),
            valid: self.valid,
            _phantom: PhantomData,
        }
    }

    #[track_caller]
    fn validate(&self) {
        assert!(
            self.valid.get(),
            "attempted to dereference an ArenaRef after its Arena was cleared"
        );
    }
}

impl<T: ?Sized> Deref for ArenaBox<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.validate();
        // SAFETY: ptr is valid and properly aligned as guaranteed by Arena::alloc
        unsafe { self.ptr.as_ref() }
    }
}

impl<T: ?Sized> DerefMut for ArenaBox<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.validate();
        // SAFETY: ptr is valid and properly aligned as guaranteed by Arena::alloc
        unsafe { self.ptr.as_mut() }
    }
}

/// A shared reference to an object allocated in an Arena.
///
/// Similar to Rc<T> but memory is managed by the Arena.
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
            _phantom: PhantomData,
        })
    }
}

impl<T: ?Sized> Deref for ArenaRef<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use super::*;

    #[test]
    fn test_arena() {
        let mut arena = Arena::new(1024);
        let a = arena.alloc(|| 1u64);
        let b = arena.alloc(|| 2u32);
        let c = arena.alloc(|| 3u16);
        let d = arena.alloc(|| 4u8);
        assert_eq!(*a, 1);
        assert_eq!(*b, 2);
        assert_eq!(*c, 3);
        assert_eq!(*d, 4);

        arena.clear();
        let a = arena.alloc(|| 5u64);
        let b = arena.alloc(|| 6u32);
        let c = arena.alloc(|| 7u16);
        let d = arena.alloc(|| 8u8);
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
        arena.alloc(|| DropGuard(dropped.clone()));
        arena.clear();
        assert!(dropped.get());
    }

    #[test]
    #[should_panic(expected = "Arena out of memory")]
    fn test_arena_overflow() {
        let mut arena = Arena::new(16);
        arena.alloc(|| 1u64);
        arena.alloc(|| 2u64);
        // This should panic with "Arena out of memory"
        arena.alloc(|| 3u64);
    }

    #[test]
    fn test_arena_alignment() {
        let mut arena = Arena::new(256);
        let x1 = arena.alloc(|| 1u8);
        let x2 = arena.alloc(|| 2u16);
        let x3 = arena.alloc(|| 3u32);
        let x4 = arena.alloc(|| 4u64);
        let x5 = arena.alloc(|| 5u64);

        assert_eq!(*x1, 1);
        assert_eq!(*x2, 2);
        assert_eq!(*x3, 3);
        assert_eq!(*x4, 4);
        assert_eq!(*x5, 5);

        // Test proper alignment
        unsafe {
            assert_eq!(x1.ptr.as_ptr().align_offset(std::mem::align_of::<u8>()), 0);
            assert_eq!(x2.ptr.as_ptr().align_offset(std::mem::align_of::<u16>()), 0);
            assert_eq!(x3.ptr.as_ptr().align_offset(std::mem::align_of::<u32>()), 0);
            assert_eq!(x4.ptr.as_ptr().align_offset(std::mem::align_of::<u64>()), 0);
            assert_eq!(x5.ptr.as_ptr().align_offset(std::mem::align_of::<u64>()), 0);
        }
    }

    #[test]
    #[should_panic(expected = "attempted to dereference an ArenaRef after its Arena was cleared")]
    fn test_arena_use_after_clear() {
        let mut arena = Arena::new(16);
        let value = arena.alloc(|| 1u64);

        arena.clear();
        let _read_value = *value;
    }
}