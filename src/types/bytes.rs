//! 定容字节容器：栈上（`StackBytes`）与堆上（`HeapBytes`，需 `alloc`）实现，`Drop` 与显式擦除均经易失写清零。
//!
//! 需要启用 `"types-bytes"` 特性。
#![allow(unused_qualifications)]
#![allow(clippy::similar_names)]
#![allow(unused)]

#[cfg(feature = "alloc")]
extern crate alloc;

use core::fmt;
use core::hint::black_box;
use core::sync::atomic::{Ordering, compiler_fence};

/// 字节容器的错误类型。未来可能新增变体/字段，请勿依赖穷尽匹配。
///
/// # Feature Requirement
///
/// 需要启用 `"types-bytes"` 特性。
///
/// # Examples
///
/// ```rust
/// use lib_unknown::types::bytes::{Bytes, StackBytes};
///
/// let mut b = StackBytes::<4>::new();
/// let err = b.extend_from_slice(b"toolong").unwrap_err();
/// assert!(matches!(err, lib_unknown::types::bytes::BytesError::CapacityExceeded { .. }));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BytesError {
    /// 请求长度超出容器容量。
    CapacityExceeded {
        /// 请求的总长度（字节）。
        requested: usize,
        /// 容器支持的最大长度（字节）。
        max: usize,
    },
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

/// 字节容器操作的统一返回类型。
///
/// # Feature Requirement
///
/// 需要启用 `"types-bytes"` 特性。
///
/// # Errors
///
/// - 当请求长度超出容器容量时返回 [`BytesError::CapacityExceeded`]。
pub type BytesResult<T = ()> = Result<T, BytesError>;

#[inline(always)]
pub(crate) fn volatile_zero(buf: &mut [u8]) {
    for byte in black_box(buf.iter_mut()) {
        // SAFETY: `byte` 为 `buf` 的独占借用派生的有效可写引用，`write_volatile` 写入对齐的 `u8`。
        unsafe {
            let _: () = core::ptr::write_volatile(black_box(byte), 0);
            black_box(());
        }
    }
    compiler_fence(Ordering::SeqCst);
}

/// 定容字节容器的统一接口：写入、擦除与向更大容器搬运。
///
/// 所有实现（[`StackBytes`] / `HeapBytes`）在 `Drop` 与显式擦除时经易失写清零。
///
/// # Feature Requirement
///
/// 需要启用 `"types-bytes"` 特性。
///
/// # Examples
///
/// ```rust
/// use lib_unknown::types::bytes::{Bytes, StackBytes};
///
/// let mut b = StackBytes::<8>::new();
/// b.extend_from_slice(b"hi").unwrap();
/// assert_eq!(b.as_slice(), b"hi");
/// b.wipe_data();
/// assert!(b.is_empty());
/// ```
pub trait Bytes:
    core::ops::Deref<Target = [u8]> + core::ops::DerefMut + AsRef<[u8]> + AsMut<[u8]> + Default + Sized
{
    /// 返回容器容量（字节），与已写入长度无关。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-bytes"` 特性。
    fn capacity(&self) -> usize;
    /// 返回已写入数据的长度（字节）。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-bytes"` 特性。
    fn len(&self) -> usize;

    /// 已写入数据是否为空。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-bytes"` 特性。
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 追加 `other` 的全部字节。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-bytes"` 特性。
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_unknown::types::bytes::{Bytes, StackBytes};
    ///
    /// let mut b = StackBytes::<8>::new();
    /// b.extend_from_slice(b"hi").unwrap();
    /// assert_eq!(b.len(), 2);
    /// ```
    ///
    /// # Errors
    ///
    /// - 当追加后总长度超出容量时返回 [`BytesError::CapacityExceeded`]，容器内容不变。
    fn extend_from_slice(&mut self, other: &[u8]) -> BytesResult;

    /// 清零全部底层存储并将长度置 0。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-bytes"` 特性。
    fn wipe_data(&mut self);

    /// 以不可变切片查看已写入数据。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-bytes"` 特性。
    fn as_slice(&self) -> &[u8] {
        self.as_ref()
    }

    /// 以可变切片访问已写入数据（不改变长度）。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-bytes"` 特性。
    fn as_mut_slice(&mut self) -> &mut [u8] {
        self.as_mut()
    }

    /// 将自身内容与 `other` 先后写入新容器，并擦除自身。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-bytes"` 特性。
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_unknown::types::bytes::{Bytes, StackBytes};
    ///
    /// let mut b = StackBytes::<8>::new();
    /// b.extend_from_slice(b"hi").unwrap();
    /// let c: StackBytes<8> = b.extend_into(b"!").unwrap();
    /// assert_eq!(c.as_slice(), b"hi!");
    /// ```
    ///
    /// # Errors
    ///
    /// - 当任一写入超出目标容器容量时返回 [`BytesError::CapacityExceeded`]。
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

    /// 将 `source` 内容追加到自身，成功后清零 `source`；失败时 `source` 保持不变。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-bytes"` 特性。
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_unknown::types::bytes::{Bytes, StackBytes};
    ///
    /// let mut b = StackBytes::<8>::new();
    /// let mut secret = *b"hi";
    /// b.extend_and_wipe(&mut secret).unwrap();
    /// assert_eq!(secret, [0u8; 2]);
    /// ```
    ///
    /// # Errors
    ///
    /// - 当追加后总长度超出容量时返回 [`BytesError::CapacityExceeded`]。
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
            /// 将内容搬运到容量为 `M` 的同类容器中。
            ///
            /// # Feature Requirement
            ///
            /// 需要启用 `"types-bytes"` 特性（`HeapBytes` 相关还需 `"alloc"` 特性）。
            ///
            /// # Examples
            ///
            /// ```rust
            /// use lib_unknown::types::bytes::StackBytes;
            ///
            /// let mut b = StackBytes::<4>::new();
            /// b.extend_from_slice(b"hi").unwrap();
            /// let c = b.try_grow::<8>().unwrap();
            /// assert_eq!(c.capacity(), 8);
            /// ```
            ///
            /// # Errors
            ///
            /// - 当已有长度超出 `M` 时返回 [`BytesError::CapacityExceeded`]。
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

            /// 返回容器容量（字节），恒为 `N`。
            ///
            /// # Feature Requirement
            ///
            /// 需要启用 `"types-bytes"` 特性（`HeapBytes` 相关还需 `"alloc"` 特性）。
            #[inline(always)]
            pub fn capacity(&self) -> usize {
                N
            }

            /// 返回已写入数据的长度（字节）。
            ///
            /// # Feature Requirement
            ///
            /// 需要启用 `"types-bytes"` 特性（`HeapBytes` 相关还需 `"alloc"` 特性）。
            #[inline(always)]
            pub fn len(&self) -> usize {
                self.len
            }

            /// 已写入数据是否为空。
            ///
            /// # Feature Requirement
            ///
            /// 需要启用 `"types-bytes"` 特性（`HeapBytes` 相关还需 `"alloc"` 特性）。
            #[inline(always)]
            pub fn is_empty(&self) -> bool {
                self.len == 0
            }

            /// 以不可变切片查看已写入数据。
            ///
            /// # Feature Requirement
            ///
            /// 需要启用 `"types-bytes"` 特性（`HeapBytes` 相关还需 `"alloc"` 特性）。
            #[inline(always)]
            pub fn as_slice(&self) -> &[u8] {
                self.as_ref()
            }

            /// 以可变切片访问已写入数据（不改变长度）。
            ///
            /// # Feature Requirement
            ///
            /// 需要启用 `"types-bytes"` 特性（`HeapBytes` 相关还需 `"alloc"` 特性）。
            #[inline(always)]
            pub fn as_mut_slice(&mut self) -> &mut [u8] {
                self.as_mut()
            }

            /// 追加 `other` 的全部字节。
            ///
            /// # Feature Requirement
            ///
            /// 需要启用 `"types-bytes"` 特性（`HeapBytes` 相关还需 `"alloc"` 特性）。
            ///
            /// # Examples
            ///
            /// ```rust
            /// use lib_unknown::types::bytes::StackBytes;
            ///
            /// let mut b = StackBytes::<4>::new();
            /// b.extend_from_slice(b"hi").unwrap();
            /// assert!(b.extend_from_slice(b"toolong").is_err());
            /// ```
            ///
            /// # Errors
            ///
            /// - 当追加后总长度超出容量时返回 [`BytesError::CapacityExceeded`]，容器内容不变。
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

            /// 清零全部底层存储并将长度置 0。
            ///
            /// # Feature Requirement
            ///
            /// 需要启用 `"types-bytes"` 特性（`HeapBytes` 相关还需 `"alloc"` 特性）。
            pub fn wipe_data(&mut self) {
                volatile_zero(&mut self.data[..]);
                self.len = 0;
            }

            /// 将自身内容与 `other` 先后写入新容器，并擦除自身。
            ///
            /// # Feature Requirement
            ///
            /// 需要启用 `"types-bytes"` 特性（`HeapBytes` 相关还需 `"alloc"` 特性）。
            ///
            /// # Examples
            ///
            /// ```rust
            /// use lib_unknown::types::bytes::StackBytes;
            ///
            /// let mut b = StackBytes::<8>::new();
            /// b.extend_from_slice(b"hi").unwrap();
            /// let c: StackBytes<8> = b.extend_into(b"!").unwrap();
            /// assert_eq!(c.as_slice(), b"hi!");
            /// ```
            ///
            /// # Errors
            ///
            /// - 当任一写入超出目标容器容量时返回 [`BytesError::CapacityExceeded`]。
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

            /// 将 `source` 内容追加到自身，成功后清零 `source`；失败时 `source` 保持不变。
            ///
            /// # Feature Requirement
            ///
            /// 需要启用 `"types-bytes"` 特性（`HeapBytes` 相关还需 `"alloc"` 特性）。
            ///
            /// # Examples
            ///
            /// ```rust
            /// use lib_unknown::types::bytes::StackBytes;
            ///
            /// let mut b = StackBytes::<8>::new();
            /// let mut src = *b"hi";
            /// b.extend_and_wipe(&mut src).unwrap();
            /// assert_eq!(src, [0u8; 2]);
            /// ```
            ///
            /// # Errors
            ///
            /// - 当追加后总长度超出容量时返回 [`BytesError::CapacityExceeded`]。
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

/// 栈上定容字节容器，`Drop` 时自动清零，适用于短密钥、非对称 nonce 等小敏感数据。
///
/// # Feature Requirement
///
/// 需要启用 `"types-bytes"` 特性。
///
/// # Examples
///
/// ```rust
/// use lib_unknown::types::bytes::{Bytes, StackBytes};
///
/// let mut b = StackBytes::<16>::new();
/// b.extend_from_slice(b"secret").unwrap();
/// assert_eq!(b.len(), 6);
/// ```
pub struct StackBytes<const N: usize> {
    len: usize,
    data: [u8; N],
}
impl<const N: usize> StackBytes<N> {
    /// 创建长度为 0、内容全零的容器。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-bytes"` 特性。
    pub const fn new() -> Self {
        Self {
            len: 0,
            data: [0u8; N],
        }
    }
}
impl_secure_buffer!(StackBytes);

/// 堆上定容字节容器，语义与 [`StackBytes`] 一致，适用于超过栈承载的较大敏感数据。
///
/// # Feature Requirement
///
/// 需要启用 `"types-bytes"` 与 `"alloc"` 特性。
///
/// # Examples
///
/// ```rust
/// use lib_unknown::types::bytes::{Bytes, HeapBytes};
///
/// let mut b = HeapBytes::<64>::new();
/// b.extend_from_slice(b"secret").unwrap();
/// assert_eq!(b.len(), 6);
/// ```
#[cfg(feature = "alloc")]
pub struct HeapBytes<const N: usize> {
    len: usize,
    data: alloc::boxed::Box<[u8; N]>,
}

#[cfg(feature = "alloc")]
fn zeroed_box<const N: usize>() -> alloc::boxed::Box<[u8; N]> {
    // SAFETY: `[u8; N]` 全零为合法值，`assume_init` 安全。
    unsafe { alloc::boxed::Box::<[u8; N]>::new_zeroed().assume_init() }
}

#[cfg(feature = "alloc")]
impl<const N: usize> HeapBytes<N> {
    /// 创建长度为 0、内容全零的容器。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-bytes"` 与 `"alloc"` 特性。
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
