// lib/src/types/bytes.rs
#![allow(unused_qualifications)]
#![allow(clippy::similar_names)]
#![allow(unused)]

#[cfg(feature = "alloc")]
extern crate alloc;

use core::fmt;
use core::hint::black_box;
use core::sync::atomic::{Ordering, compiler_fence};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BytesError {
    CapacityExceeded { requested: usize, max: usize },
}

impl fmt::Display for BytesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityExceeded { requested, max } => write!(
                f,
                "capacity exceeded: requested {requested} bytes, max supported {max} bytes"
            ),
        }
    }
}

impl core::error::Error for BytesError {}

pub type BytesResult<T = ()> = Result<T, BytesError>;

#[inline(always)]
pub(crate) fn volatile_zero(buf: &mut [u8]) {
    for byte in black_box(buf.iter_mut()) {
        unsafe {
            let _: () = core::ptr::write_volatile(black_box(byte), 0);
            black_box(());
        }
    }
    compiler_fence(Ordering::SeqCst);
}

pub trait Bytes:
    core::ops::Deref<Target = [u8]> + core::ops::DerefMut + AsRef<[u8]> + AsMut<[u8]> + Default + Sized
{
    fn capacity(&self) -> usize;
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn extend_from_slice(&mut self, other: &[u8]) -> BytesResult;

    fn wipe_data(&mut self);

    fn as_slice(&self) -> &[u8] {
        self.as_ref()
    }

    fn as_mut_slice(&mut self) -> &mut [u8] {
        self.as_mut()
    }

    fn extend_into<T>(mut self, other: &[u8]) -> BytesResult<T>
    where
        Self: Sized,
        T: Bytes + Default,
    {
        let mut new_buf = T::default();
        new_buf.extend_from_slice(self.as_slice())?;
        new_buf.extend_from_slice(other)?;
        self.wipe_data();

        Ok(new_buf)
    }

    fn extend_and_wipe<T>(&mut self, mut source: T) -> BytesResult
    where
        Self: Sized,
        T: AsRef<[u8]> + AsMut<[u8]>,
    {
        let res = self.extend_from_slice(source.as_ref());
        if res.is_ok() {
            volatile_zero(source.as_mut());
        }
        res
    }
}

macro_rules! impl_secure_buffer {
    ($name:ident) => {
        impl<const N: usize> $name<N> {
            pub fn try_grow<const M: usize>(self) -> BytesResult<$name<M>> {
                if self.len > M {
                    return Err(BytesError::CapacityExceeded {
                        requested: self.len,
                        max: M,
                    });
                }
                let mut out = $name::<M>::new();
                out.data[..self.len].copy_from_slice(&self.data[..self.len]);
                out.len = self.len;
                Ok(out)
            }

            #[inline(always)]
            pub fn capacity(&self) -> usize {
                N
            }

            #[inline(always)]
            pub fn len(&self) -> usize {
                self.len
            }

            #[inline(always)]
            pub fn is_empty(&self) -> bool {
                self.len == 0
            }

            #[inline(always)]
            pub fn as_slice(&self) -> &[u8] {
                self.as_ref()
            }

            #[inline(always)]
            pub fn as_mut_slice(&mut self) -> &mut [u8] {
                self.as_mut()
            }

            pub fn extend_from_slice(&mut self, other: &[u8]) -> BytesResult {
                let new_len =
                    self.len
                        .checked_add(other.len())
                        .ok_or(BytesError::CapacityExceeded {
                            requested: usize::MAX,
                            max: N,
                        })?;
                if new_len > N {
                    return Err(BytesError::CapacityExceeded {
                        requested: new_len,
                        max: N,
                    });
                }
                self.data[self.len..new_len].copy_from_slice(other);
                self.len = new_len;
                Ok(())
            }

            pub fn wipe_data(&mut self) {
                volatile_zero(&mut self.data[..]);
                self.len = 0;
            }

            pub fn extend_into<T>(mut self, other: &[u8]) -> BytesResult<T>
            where
                T: Bytes + Default,
            {
                let mut new_buf = T::default();
                new_buf.extend_from_slice(self.as_slice())?;
                new_buf.extend_from_slice(other)?;
                self.wipe_data();
                Ok(new_buf)
            }

            pub fn extend_and_wipe<T>(&mut self, mut source: T) -> BytesResult
            where
                T: AsRef<[u8]> + AsMut<[u8]>,
            {
                let res = self.extend_from_slice(source.as_ref());
                if res.is_ok() {
                    volatile_zero(source.as_mut());
                }
                res
            }
        }

        impl<const N: usize> Default for $name<N> {
            fn default() -> Self {
                Self::new()
            }
        }

        impl<const N: usize> Drop for $name<N> {
            fn drop(&mut self) {
                volatile_zero(&mut self.data[..]);
            }
        }

        impl<const N: usize> AsRef<[u8]> for $name<N> {
            fn as_ref(&self) -> &[u8] {
                &self.data[..self.len]
            }
        }

        impl<const N: usize> AsMut<[u8]> for $name<N> {
            fn as_mut(&mut self) -> &mut [u8] {
                &mut self.data[..self.len]
            }
        }

        impl<const N: usize> core::ops::Deref for $name<N> {
            type Target = [u8];
            fn deref(&self) -> &Self::Target {
                &self.data[..self.len]
            }
        }

        impl<const N: usize> core::ops::DerefMut for $name<N> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.data[..self.len]
            }
        }

        impl<const N: usize> fmt::Debug for $name<N> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct(stringify!($name))
                    .field("len", &self.len)
                    .field("capacity", &N)
                    .finish()
            }
        }

        impl<const N: usize> Bytes for $name<N> {
            #[inline(always)]
            fn capacity(&self) -> usize {
                self.capacity()
            }
            #[inline(always)]
            fn len(&self) -> usize {
                self.len()
            }
            #[inline(always)]
            fn is_empty(&self) -> bool {
                self.is_empty()
            }
            #[inline(always)]
            fn as_slice(&self) -> &[u8] {
                self.as_slice()
            }
            #[inline(always)]
            fn as_mut_slice(&mut self) -> &mut [u8] {
                self.as_mut_slice()
            }
            #[inline(always)]
            fn extend_from_slice(&mut self, other: &[u8]) -> BytesResult {
                self.extend_from_slice(other)
            }
            #[inline(always)]
            fn wipe_data(&mut self) {
                self.wipe_data()
            }
            #[inline(always)]
            fn extend_into<T>(self, other: &[u8]) -> BytesResult<T>
            where
                Self: Sized,
                T: Bytes + Default,
            {
                self.extend_into(other)
            }
            #[inline(always)]
            fn extend_and_wipe<T>(&mut self, source: T) -> BytesResult
            where
                Self: Sized,
                T: AsRef<[u8]> + AsMut<[u8]>,
            {
                self.extend_and_wipe(source)
            }
        }
    };
}

pub struct StackBytes<const N: usize> {
    len: usize,
    data: [u8; N],
}
impl<const N: usize> StackBytes<N> {
    pub const fn new() -> Self {
        Self {
            len: 0,
            data: [0u8; N],
        }
    }
}
impl_secure_buffer!(StackBytes);

#[cfg(feature = "alloc")]
pub struct HeapBytes<const N: usize> {
    len: usize,
    data: alloc::boxed::Box<[u8; N]>,
}

#[cfg(feature = "alloc")]
fn zeroed_box<const N: usize>() -> alloc::boxed::Box<[u8; N]> {
    unsafe { alloc::boxed::Box::<[u8; N]>::new_zeroed().assume_init() }
}

#[cfg(feature = "alloc")]
impl<const N: usize> HeapBytes<N> {
    pub fn new() -> Self {
        Self {
            len: 0,
            data: zeroed_box::<N>(),
        }
    }
}
#[cfg(feature = "alloc")]
impl_secure_buffer!(HeapBytes);

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_buffer() {
        let mut d = StackBytes::<32>::new();

        let mut secret_data = *b"super_secret_password";
        d.extend_and_wipe(&mut secret_data).unwrap();

        assert_eq!(&secret_data, &[0u8; 21]);
        assert_eq!(d.as_slice(), b"super_secret_password");
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_heap_buffer() {
        let mut d = StackBytes::<32>::new();

        let mut secret_data = *b"super_secret_password";
        d.extend_and_wipe(&mut secret_data).unwrap();

        let extra_data = b"_suffix";
        let mut x = d.extend_into::<HeapBytes<512>>(extra_data).unwrap();

        fn aaa(d: &impl Bytes) {
            assert!(d.capacity() >= 512);
        }
        aaa(&x);

        assert_eq!(x.as_slice(), b"super_secret_password_suffix");
        assert!(x.starts_with(b"super_secret"));

        x[0] = b'S';
        assert_eq!(x.as_slice(), b"Super_secret_password_suffix");

        fn requires_slice(s: &[u8]) {
            assert_eq!(s.len(), 28);
        }
        requires_slice(&x);

        fn bbb(d: impl Bytes) {
            assert!(d.capacity() >= 512);
        }
        bbb(x);
    }
}
