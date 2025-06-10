use std::mem::MaybeUninit;

use facet::{Facet, PtrConst, PtrMut, PtrUninit, Shape};

type InlineStorage = usize;

pub union ErasedUninit {
    inline: MaybeUninit<InlineStorage>,
    /// a pointer to the value allocated on the heap
    boxed: PtrUninit<'static>,
}

impl ErasedUninit {
    fn as_ptr<'a>(&mut self, shape: &Shape) -> PtrUninit<'_> {
        match ErasedStorage::for_shape(shape) {
            ErasedStorage::Inline => PtrUninit::new(unsafe { self.inline.as_mut_ptr() }),
            ErasedStorage::Boxed => unsafe { self.boxed },
        }
    }

    /// Safety: Assumes that it is initialized
    unsafe fn as_const_ptr_assume_init(&self, shape: &Shape) -> PtrConst<'_> {
        match ErasedStorage::for_shape(shape) {
            ErasedStorage::Inline => PtrConst::new(unsafe { self.inline.as_ptr() }),
            ErasedStorage::Boxed => unsafe { self.boxed.assume_init().as_const() },
        }
    }

    pub unsafe fn assume_init(self) -> Erased {
        Erased(self)
    }
}

#[repr(transparent)]
pub struct Erased(ErasedUninit);

impl Erased {
    pub fn uninit(shape: &Shape) -> ErasedUninit {
        match ErasedStorage::for_shape(shape) {
            ErasedStorage::Inline => ErasedUninit {
                inline: MaybeUninit::uninit(),
            },
            ErasedStorage::Boxed => {
                let ptr = unsafe { std::alloc::alloc(shape.layout.sized_layout().unwrap()) };
                ErasedUninit {
                    boxed: PtrUninit::new(ptr),
                }
            }
        }
    }

    pub fn new<'a, T: 'a>(value: T) -> Self
    where
        T: Facet<'a>,
    {
        let mut uninit = Self::uninit(T::SHAPE);

        unsafe {
            {
                let ptr = uninit.as_ptr(T::SHAPE);
                ptr.put(value);
            }
            uninit.assume_init()
        }
    }

    /// Safety: must be correct shape for T
    pub unsafe fn as_ptr<'a>(&'a self, shape: &Shape) -> PtrConst<'a> {
        unsafe { self.0.as_const_ptr_assume_init(shape) }
    }

    /// Safety: must be correct shape for T
    pub unsafe fn as_mut_ptr<'a>(&'a mut self, shape: &Shape) -> PtrMut<'a> {
        unsafe { self.0.as_ptr(shape).assume_init() }
    }

    /// Safety: T must be the correct type
    pub unsafe fn into_typed<'a, T: Facet<'a>>(self) -> T {
        unsafe { self.as_ptr(T::SHAPE).read() }
    }
}

#[derive(Clone, Copy)]
enum ErasedStorage {
    Inline,
    Boxed,
}

impl ErasedStorage {
    fn for_shape(shape: &Shape) -> Self {
        match shape.layout {
            facet::ShapeLayout::Sized(layout)
                if layout.size() <= std::mem::size_of::<InlineStorage>()
                    && layout.align() <= std::mem::align_of::<InlineStorage>() =>
            {
                Self::Inline
            }
            _ => Self::Boxed,
        }
    }
}
