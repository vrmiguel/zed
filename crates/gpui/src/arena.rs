use std::{
    alloc::{self, Layout},
    cell::Cell,
    ops::{Deref, DerefMut},
    ptr::{self, NonNull},
    rc::Rc,
};

struct ArenaElement {
    value: NonNull<u8>,
    drop: unsafe fn(NonNull<u8>),
}

impl Drop for ArenaElement {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe {
            (self.drop)(self.value);
        }
    }
}

pub struct Arena {
    start: NonNull<u8>,
    end: NonNull<u8>,
    offset: NonNull<u8>,
    elements: Vec<ArenaElement>,
    valid: Rc<Cell<bool>>,
    layout: Layout,
}

impl Arena {
    pub fn new(size_in_bytes: usize) -> Result<Self, alloc::LayoutError> {
        unsafe {
            let layout = Layout::from_size_align(size_in_bytes, std::mem::align_of::<usize>())?;
            let start = NonNull::new(alloc::alloc(layout))
                .ok_or(alloc::LayoutError)?;
            let end = NonNull::new(start.as_ptr().add(size_in_bytes))
                .ok_or(alloc::LayoutError)?;
            Ok(Self {
                start,
                end,
                offset: start,
                elements: Vec::new(),
                valid: Rc::new(Cell::new(true)),
                layout,
            })
        }
    }

    pub fn len(&self) -> usize {
        unsafe { self.offset.as_ptr().offset_from(self.start.as_ptr()) as usize }
    }

    pub fn capacity(&self) -> usize {
        unsafe { self.end.as_ptr().offset_from(self.start.as_ptr()) as usize }
    }

    pub fn clear(&mut self) {
        self.valid.set(false);
        self.valid = Rc::new(Cell::new(true));
        self.elements.clear();
        self.offset = self.start;
    }

    #[inline(always)]
    pub fn alloc<T>(&mut self, f: impl FnOnce() -> T) -> Result<ArenaBox<T>, &'static str> {
        #[inline(always)]
        unsafe fn inner_writer<T, F>(ptr: NonNull<T>, f: F)
        where
            F: FnOnce() -> T,
        {
            ptr::write(ptr.as_ptr(), f());
        }

        unsafe fn drop<T>(ptr: NonNull<u8>) {
            ptr::drop_in_place(ptr.cast::<T>().as_ptr());
        }

        unsafe {
            let layout = Layout::new::<T>();
            let aligned_offset = self.offset.as_ptr().add(
                self.offset.as_ptr().align_offset(layout.align())
            );
            let offset = NonNull::new(aligned_offset)
                .ok_or("alignment calculation overflow")?;
            let next_offset = NonNull::new(offset.as_ptr().add(layout.size()))
                .ok_or("allocation size overflow")?;
            
            if next_offset.as_ptr() > self.end.as_ptr() {
                return Err("not enough space in Arena");
            }

            let ptr = NonNull::new(offset.as_ptr() as *mut T)
                .ok_or("pointer cast failed")?;
            let result = ArenaBox {
                ptr,
                valid: self.valid.clone(),
            };

            inner_writer(ptr, f);
            self.elements.push(ArenaElement {
                value: offset,
                drop: drop::<T>,
            });
            self.offset = next_offset;

            Ok(result)
        }
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        unsafe {
            self.clear();
            alloc::dealloc(self.start.as_ptr(), self.layout);
        }
    }
}

pub struct ArenaBox<T: ?Sized> {
    ptr: NonNull<T>,
    valid: Rc<Cell<bool>>,
}

impl<T: ?Sized> ArenaBox<T> {
    #[inline(always)]
    pub fn map<U: ?Sized>(mut self, f: impl FnOnce(&mut T) -> &mut U) -> ArenaBox<U> {
        let ptr = NonNull::from(f(&mut self));
        ArenaBox {
            ptr,
            valid: self.valid,
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
        unsafe { self.ptr.as_ref() }
    }
}

impl<T: ?Sized> DerefMut for ArenaBox<T> {
    #[inline(always)]
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
        assert!(arena.alloc(|| 3u64).is_err());
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

        unsafe {
            assert_eq!(x1.ptr.as_ptr().align_offset(std::mem::align_of_val(&*x1)), 0);
            assert_eq!(x2.ptr.as_ptr().align_offset(std::mem::align_of_val(&*x2)), 0);
        }
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