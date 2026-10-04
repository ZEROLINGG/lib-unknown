//! 定容 UTF-8 字符串容器：栈上（`StackStr`）与堆上（`HeapStr`，需 `alloc`）实现，写入时校验编码，源缓冲按需擦除。
//!
//! 需要启用 `"types-str"` 特性。
#![allow(unused_qualifications)]
#![allow(clippy::similar_names)]
#![allow(unused)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
use super::bytes::HeapBytes;
use super::bytes::{Bytes, BytesError, BytesResult, StackBytes, volatile_zero};
use core::convert::TryFrom;
use core::str::Utf8Error;

/// 安全字符串的错误类型。
///
/// # Feature Requirement
///
/// 需要启用 `"types-str"` 特性。
///
/// # Examples
///
/// ```rust
/// use core::convert::TryInto;
/// use lib_unknown::types::str::{StackStr, StrError};
///
/// let res: Result<StackStr<2>, _> = "toolong".try_into();
/// assert!(matches!(res, Err(StrError::CapacityExceeded(_))));
/// ```
#[derive(Debug)]
pub enum StrError {
    /// 长度超出容器容量。
    CapacityExceeded(BytesError),
    /// 源内容不是合法 UTF-8（源仍会被清零）。
    InvalidUtf8(Utf8Error),
    /// 收到空指针。
    NullPointer,
}

impl core::error::Error for StrError {}

impl core::fmt::Display for StrError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::CapacityExceeded(e) => write!(f, "capacity exceeded: {e}"),
            Self::InvalidUtf8(e) => write!(f, "invalid UTF-8 sequence: {e}"),
            Self::NullPointer => write!(f, "received null pointer"),
        }
    }
}

#[inline(always)]
fn safe_parse_and_wipe<F>(buffer: &mut [u8], mut push_fn: F) -> Result<(), StrError>
where
    F: FnMut(&str) -> Result<(), StrError>,
{
    let res = match ::core::str::from_utf8(buffer) {
        Ok(valid_str) => push_fn(valid_str),
        Err(e) => Err(StrError::InvalidUtf8(e)),
    };
    volatile_zero(buffer);
    res
}

/// 安全字符串的统一接口：只读视图，内部恒为合法 UTF-8。
///
/// # Feature Requirement
///
/// 需要启用 `"types-str"` 特性。
///
/// # Examples
///
/// ```rust
/// use core::convert::TryInto;
/// use lib_unknown::types::str::{StackStr, Str};
///
/// let s: StackStr<8> = "hi".try_into().unwrap();
/// assert_eq!(s.as_str(), "hi");
/// assert_eq!(s.len(), 2);
/// ```
pub trait Str:
    ::core::ops::Deref<Target = str> + ::core::fmt::Display + ::core::fmt::Debug + Send + Sync
{
    /// 以 `&str` 形式查看内容。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-str"` 特性。
    fn as_str(&self) -> &str;

    /// 以字节切片查看内容。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-str"` 特性。
    #[inline(always)]
    fn as_bytes(&self) -> &[u8] {
        self.as_str().as_bytes()
    }

    /// 类型擦除的堆克隆，用于 `Box<dyn Str>` 容器。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-str"` 与 `"alloc"` 特性。
    #[cfg(feature = "alloc")]
    fn dyn_clone(&self) -> alloc::boxed::Box<dyn Str>;

    /// 返回内容的字节长度。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-str"` 特性。
    #[inline(always)]
    fn len(&self) -> usize {
        self.as_str().len()
    }

    /// 内容是否为空。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-str"` 特性。
    #[inline(always)]
    fn is_empty(&self) -> bool {
        self.as_str().is_empty()
    }
}

/// 栈上定容安全字符串，`Drop` 时自动清零，适用于短口令、令牌等小敏感文本。
///
/// # Feature Requirement
///
/// 需要启用 `"types-str"` 特性。
///
/// # Examples
///
/// ```rust
/// use core::convert::TryInto;
/// use lib_unknown::types::str::StackStr;
///
/// let s: StackStr<8> = "hi".try_into().unwrap();
/// assert_eq!(&*s, "hi");
/// ```
pub struct StackStr<const N: usize>(StackBytes<N>);

/// 堆上定容安全字符串，语义与 [`StackStr`] 一致，适用于较长的敏感文本。
///
/// # Feature Requirement
///
/// 需要启用 `"types-str"` 与 `"alloc"` 特性。
///
/// # Examples
///
/// ```rust
/// use core::convert::TryInto;
/// use lib_unknown::types::str::HeapStr;
///
/// let s: HeapStr<16> = "hi".try_into().unwrap();
/// assert_eq!(&*s, "hi");
/// ```
#[cfg(feature = "alloc")]
pub struct HeapStr<const N: usize>(HeapBytes<N>);

impl<const N: usize> StackStr<N> {
    /// 容器容量（字节）。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-str"` 特性。
    pub const CAPACITY: usize = N;
    /// 创建空字符串。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-str"` 特性。
    #[inline(always)]
    pub const fn new() -> Self {
        Self(StackBytes::new())
    }
}

#[cfg(feature = "alloc")]
impl<const N: usize> HeapStr<N> {
    /// 容器容量（字节）。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-str"` 与 `"alloc"` 特性。
    pub const CAPACITY: usize = N;
    /// 创建空字符串。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-str"` 与 `"alloc"` 特性。
    #[inline(always)]
    pub fn new() -> Self {
        Self(HeapBytes::new())
    }
}

macro_rules! impl_secure_str_base {
    ($name:ident) => {
        impl<const N: usize> $name<N> {
            /// 从原始可变指针拷贝并擦除源缓冲构造安全字符串。
            ///
            /// 成功时将源 `len` 字节按 UTF-8 解析后拷贝并清零源内存。
            ///
            /// # Safety
            ///
            /// 调用此函数必须保证：
            /// 1. `ptr` 非空且在 `len` 范围内可读写、正确对齐；
            /// 2. 调用期间该内存不被其他线程并发访问。
            ///
            /// # Errors
            ///
            /// - `ptr` 为空时返回 [`StrError::NullPointer`]。
            /// - 源内容非合法 UTF-8 时返回 [`StrError::InvalidUtf8`]（源仍会被清零）。
            /// - 长度超出 `N` 时返回 [`StrError::CapacityExceeded`]。
            pub unsafe fn from_raw_parts_mut(ptr: *mut u8, len: usize) -> Result<Self, StrError> {
                if ptr.is_null() {
                    return Err(StrError::NullPointer);
                }
                if len == 0 {
                    return Ok(Self::new());
                }

                // SAFETY: `ptr` 非空已检查；有效性、对齐与独占性由本函数的 `# Safety` 约定保证。
                let slice = unsafe { ::core::slice::from_raw_parts_mut(ptr, len) };

                let mut out = Self::new();
                safe_parse_and_wipe(slice, |s| out.try_push_str(s))?;
                Ok(out)
            }

            #[inline(always)]
            fn try_push_str(&mut self, s: &str) -> Result<(), StrError> {
                self.0
                    .extend_from_slice(s.as_bytes())
                    .map_err(StrError::CapacityExceeded)
            }

            /// 将内容搬运到容量为 `M` 的同类字符串中。
            ///
            /// # Feature Requirement
            ///
            /// 需要启用 `"types-str"` 特性（`HeapStr` 相关还需 `"alloc"` 特性）。
            ///
            /// # Examples
            ///
            /// ```rust
            /// use core::convert::TryInto;
            /// use lib_unknown::types::str::StackStr;
            ///
            /// let s: StackStr<4> = "hi".try_into().unwrap();
            /// let c = s.try_grow::<8>().unwrap();
            /// assert_eq!(&*c, "hi");
            /// ```
            ///
            /// # Errors
            ///
            /// - 当已有长度超出 `M` 时返回 [`StrError::CapacityExceeded`]。
            ///
            /// # Panics
            ///
            /// - 内部 `expect("Capacity already checked")`：前置长度检查保证不触发，仅防御性保留。
            #[inline]
            pub fn try_grow<const M: usize>(self) -> Result<$name<M>, StrError> {
                if M < self.len() {
                    return Err(StrError::CapacityExceeded(BytesError::CapacityExceeded {
                        requested: self.len(),
                        max: M,
                    }));
                }
                let mut larger = $name::<M>::new();
                larger
                    .0
                    .extend_from_slice(self.as_bytes())
                    .expect("Capacity already checked");
                Ok(larger)
            }

            /// 将自身与 `suffix` 拼接为容量 `M` 的新字符串。
            ///
            /// # Feature Requirement
            ///
            /// 需要启用 `"types-str"` 特性（`HeapStr` 相关还需 `"alloc"` 特性）。
            ///
            /// # Examples
            ///
            /// ```rust
            /// use core::convert::TryInto;
            /// use lib_unknown::types::str::StackStr;
            ///
            /// let s: StackStr<4> = "hi".try_into().unwrap();
            /// let c = s.push_str_into::<8>("!").unwrap();
            /// assert_eq!(&*c, "hi!");
            /// ```
            ///
            /// # Errors
            ///
            /// - 当拼接后长度超出 `M` 时返回 [`StrError::CapacityExceeded`]。
            ///
            /// # Panics
            ///
            /// - 两处内部 `unwrap`：前置长度检查保证不触发，仅防御性保留。
            #[inline]
            pub fn push_str_into<const M: usize>(self, suffix: &str) -> Result<$name<M>, StrError> {
                let required_len = self.len() + suffix.len();
                if required_len > M {
                    return Err(StrError::CapacityExceeded(BytesError::CapacityExceeded {
                        requested: required_len,
                        max: M,
                    }));
                }
                let mut larger = $name::<M>::new();
                larger.0.extend_from_slice(self.as_bytes()).unwrap();
                larger.0.extend_from_slice(suffix.as_bytes()).unwrap();
                Ok(larger)
            }

            /// 将自身与 `rhs` 拼接为容量 `M` 的新字符串（`rhs` 为任意可借用为 `str` 的类型）。
            ///
            /// # Feature Requirement
            ///
            /// 需要启用 `"types-str"` 特性（`HeapStr` 相关还需 `"alloc"` 特性）。
            ///
            /// # Examples
            ///
            /// ```rust
            /// use core::convert::TryInto;
            /// use lib_unknown::types::str::StackStr;
            ///
            /// let s: StackStr<4> = "hi".try_into().unwrap();
            /// let c = s.concat_into::<8>("!").unwrap();
            /// assert_eq!(&*c, "hi!");
            /// ```
            ///
            /// # Errors
            ///
            /// - 当拼接后长度超出 `M` 时返回 [`StrError::CapacityExceeded`]。
            pub fn concat_into<const M: usize>(
                self,
                rhs: impl ::core::convert::AsRef<str>,
            ) -> Result<$name<M>, StrError> {
                let mut new_buf = $name::<M>::new();
                new_buf.try_push_str(self.as_str())?;
                new_buf.try_push_str(rhs.as_ref())?;
                Ok(new_buf)
            }
        }

        impl<const N: usize> ::core::default::Default for $name<N> {
            #[inline(always)]
            fn default() -> Self {
                Self::new()
            }
        }

        impl<const N: usize> Str for $name<N> {
            #[inline(always)]
            fn as_str(&self) -> &str {
                // SAFETY: 类型不变式保证内部字节恒为合法 UTF-8（经 `try_push_str` 校验写入）。
                unsafe { ::core::str::from_utf8_unchecked(self.0.as_slice()) }
            }

            #[cfg(feature = "alloc")]
            #[inline(always)]
            fn dyn_clone(&self) -> alloc::boxed::Box<dyn Str> {
                alloc::boxed::Box::new(self.clone())
            }
        }

        impl<const N: usize> ::core::clone::Clone for $name<N> {
            #[inline(always)]
            fn clone(&self) -> Self {
                let mut new_str = Self::new();
                let _ = new_str.0.extend_from_slice(self.as_bytes());
                new_str
            }
        }
    };
}

macro_rules! impl_try_from_traits {
    ($name:ident) => {
        impl<const N: usize> TryFrom<&str> for $name<N> {
            type Error = StrError;
            #[inline(always)]
            fn try_from(s: &str) -> Result<Self, Self::Error> {
                let mut out = Self::new();
                out.try_push_str(s)?;
                Ok(out)
            }
        }

        impl<const N: usize> ::core::str::FromStr for $name<N> {
            type Err = StrError;
            #[inline(always)]
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::try_from(s)
            }
        }

        impl<const N: usize> TryFrom<&mut [u8; N]> for $name<N> {
            type Error = StrError;
            fn try_from(arr: &mut [u8; N]) -> Result<Self, Self::Error> {
                let mut out = Self::new();
                safe_parse_and_wipe(arr, |s| out.try_push_str(s))?;
                Ok(out)
            }
        }

        impl<const N: usize> TryFrom<&mut [u8]> for $name<N> {
            type Error = StrError;
            fn try_from(arr: &mut [u8]) -> Result<Self, Self::Error> {
                let mut out = Self::new();
                safe_parse_and_wipe(arr, |s| out.try_push_str(s))?;
                Ok(out)
            }
        }

        #[cfg(feature = "alloc")]
        impl<const N: usize> TryFrom<alloc::boxed::Box<[u8; N]>> for $name<N> {
            type Error = StrError;
            fn try_from(mut b: alloc::boxed::Box<[u8; N]>) -> Result<Self, Self::Error> {
                let mut out = Self::new();
                safe_parse_and_wipe(&mut *b, |s| out.try_push_str(s))?;
                Ok(out)
            }
        }

        #[cfg(feature = "alloc")]
        impl<const N: usize> TryFrom<alloc::vec::Vec<u8>> for $name<N> {
            type Error = StrError;
            fn try_from(mut v: alloc::vec::Vec<u8>) -> Result<Self, Self::Error> {
                let mut out = Self::new();
                safe_parse_and_wipe(v.as_mut_slice(), |s| out.try_push_str(s))?;
                Ok(out)
            }
        }

        #[cfg(feature = "alloc")]
        impl<const N: usize> TryFrom<alloc::string::String> for $name<N> {
            type Error = StrError;
            #[inline(always)]
            fn try_from(s: alloc::string::String) -> Result<Self, Self::Error> {
                Self::try_from(s.into_bytes())
            }
        }
    };
}

macro_rules! __impl_common_str_traits {
    ($name:ident) => {
        impl<const N: usize> ::core::ops::Deref for $name<N> {
            type Target = str;
            #[inline(always)]
            fn deref(&self) -> &Self::Target {
                self.as_str()
            }
        }
        impl<const N: usize> ::core::fmt::Display for $name<N> {
            #[inline(always)]
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.write_str(self.as_str())
            }
        }
        impl<const N: usize> ::core::fmt::Debug for $name<N> {
            #[inline(always)]
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                ::core::fmt::Debug::fmt(self.as_str(), f)
            }
        }
        impl<const N: usize> ::core::convert::AsRef<str> for $name<N> {
            #[inline(always)]
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }
        impl<const N: usize> ::core::convert::AsRef<[u8]> for $name<N> {
            #[inline(always)]
            fn as_ref(&self) -> &[u8] {
                self.as_bytes()
            }
        }
        impl<const N: usize> ::core::cmp::PartialEq<str> for $name<N> {
            #[inline(always)]
            fn eq(&self, other: &str) -> bool {
                self.as_str() == other
            }
        }
        impl<'a, const N: usize> ::core::cmp::PartialEq<&'a str> for $name<N> {
            #[inline(always)]
            fn eq(&self, other: &&'a str) -> bool {
                self.as_str() == *other
            }
        }
        impl<const N: usize, const M: usize> ::core::cmp::PartialEq<$name<M>> for $name<N> {
            #[inline(always)]
            fn eq(&self, other: &$name<M>) -> bool {
                self.as_str() == other.as_str()
            }
        }
        #[cfg(feature = "alloc")]
        impl<const N: usize> ::core::cmp::PartialEq<alloc::string::String> for $name<N> {
            #[inline(always)]
            fn eq(&self, other: &alloc::string::String) -> bool {
                self.as_str() == other.as_str()
            }
        }
        #[cfg(feature = "alloc")]
        impl<const N: usize> ::core::cmp::PartialEq<$name<N>> for alloc::string::String {
            #[inline(always)]
            fn eq(&self, other: &$name<N>) -> bool {
                self.as_str() == other.as_str()
            }
        }
        impl<const N: usize> ::core::cmp::Eq for $name<N> {}
        impl<const N: usize> ::core::cmp::PartialOrd for $name<N> {
            #[inline(always)]
            fn partial_cmp(&self, other: &Self) -> ::core::option::Option<::core::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }
        impl<const N: usize> ::core::cmp::Ord for $name<N> {
            #[inline(always)]
            fn cmp(&self, other: &Self) -> ::core::cmp::Ordering {
                self.as_str().cmp(other.as_str())
            }
        }
        impl<const N: usize> ::core::hash::Hash for $name<N> {
            #[inline(always)]
            fn hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
                self.as_str().hash(state)
            }
        }
        impl<const N: usize> ::core::borrow::Borrow<str> for $name<N> {
            #[inline(always)]
            fn borrow(&self) -> &str {
                self.as_str()
            }
        }
    };
}

impl_secure_str_base!(StackStr);
impl_try_from_traits!(StackStr);
__impl_common_str_traits!(StackStr);

#[cfg(feature = "alloc")]
impl_secure_str_base!(HeapStr);
#[cfg(feature = "alloc")]
impl_try_from_traits!(HeapStr);
#[cfg(feature = "alloc")]
__impl_common_str_traits!(HeapStr);

#[cfg(feature = "alloc")]
impl<const N: usize> HeapStr<N> {
    /// 将内容复制为普通 `String`（敏感内存外泄，谨慎使用，已废弃）。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-str"` 与 `"alloc"` 特性。
    #[inline(always)]
    #[deprecated(note = "This leaks secure memory into alloc::string::String. Use with caution.")]
    pub fn leak_into_string(self) -> alloc::string::String {
        use alloc::borrow::ToOwned;
        self.as_str().to_owned()
    }
}

#[cfg(feature = "alloc")]
impl<const N: usize> StackStr<N> {
    /// 将栈字符串搬运为容量 `M` 的堆字符串。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-str"` 与 `"alloc"` 特性。
    ///
    /// # Examples
    ///
    /// ```rust
    /// use core::convert::TryInto;
    /// use lib_unknown::types::str::StackStr;
    ///
    /// let s: StackStr<4> = "hi".try_into().unwrap();
    /// let h = s.into_heap::<8>().unwrap();
    /// assert_eq!(&*h, "hi");
    /// ```
    ///
    /// # Errors
    ///
    /// - 当已有长度超出 `M` 时返回 [`StrError::CapacityExceeded`]。
    ///
    /// # Panics
    ///
    /// - 内部 `expect("Capacity checked")`：前置长度检查保证不触发，仅防御性保留。
    #[inline]
    pub fn into_heap<const M: usize>(self) -> Result<HeapStr<M>, StrError> {
        if M < self.len() {
            return Err(StrError::CapacityExceeded(BytesError::CapacityExceeded {
                requested: self.len(),
                max: M,
            }));
        }
        let mut heap_str = HeapStr::<M>::new();
        heap_str
            .0
            .extend_from_slice(self.as_bytes())
            .expect("Capacity checked");
        Ok(heap_str)
    }
}

#[cfg(feature = "alloc")]
impl<const N: usize, const M: usize> ::core::cmp::PartialEq<StackStr<M>> for HeapStr<N> {
    #[inline(always)]
    fn eq(&self, other: &StackStr<M>) -> bool {
        self.as_str() == other.as_str()
    }
}
#[cfg(feature = "alloc")]
impl<const N: usize, const M: usize> ::core::cmp::PartialEq<HeapStr<N>> for StackStr<M> {
    #[inline(always)]
    fn eq(&self, other: &HeapStr<N>) -> bool {
        self.as_str() == other.as_str()
    }
}

#[cfg(feature = "alloc")]
impl ::core::clone::Clone for alloc::boxed::Box<dyn Str> {
    #[inline(always)]
    fn clone(&self) -> Self {
        self.dyn_clone()
    }
}
#[cfg(feature = "alloc")]
impl ::core::cmp::PartialEq for alloc::boxed::Box<dyn Str> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}
#[cfg(feature = "alloc")]
impl ::core::cmp::Eq for alloc::boxed::Box<dyn Str> {}
#[cfg(feature = "alloc")]
impl ::core::cmp::PartialEq<str> for alloc::boxed::Box<dyn Str> {
    #[inline(always)]
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}
#[cfg(feature = "alloc")]
impl ::core::cmp::PartialEq<&str> for alloc::boxed::Box<dyn Str> {
    #[inline(always)]
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}
#[cfg(feature = "alloc")]
impl ::core::cmp::PartialEq<alloc::string::String> for alloc::boxed::Box<dyn Str> {
    #[inline(always)]
    fn eq(&self, other: &alloc::string::String) -> bool {
        self.as_str() == other.as_str()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    #[cfg(feature = "alloc")]
    use alloc::{boxed::Box, vec, vec::Vec};
    use core::convert::TryInto;

    #[test]
    fn test_stack_from_str_literal() {
        let s: StackStr<8> = "hello".try_into().unwrap();
        assert_eq!(s, "hello");
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_heap_from_str_literal() {
        let h: HeapStr<16> = "hello".try_into().unwrap();
        assert_eq!(h, "hello");
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_vec_dyn_str() {
        let mut secret_list: Vec<Box<dyn Str>> = Vec::new();

        let h: HeapStr<256> = "SuperSecret_Heap".try_into().unwrap();
        secret_list.push(Box::new(h));

        let s: StackStr<16> = "Short_Stack".try_into().unwrap();
        secret_list.push(Box::new(s));

        let s2: StackStr<32> = "Another_Stack_32".try_into().unwrap();
        secret_list.push(Box::new(s2));

        assert_eq!(secret_list.len(), 3);
        assert!(secret_list[0].starts_with("Super"));
        assert!(secret_list[1].ends_with("Stack"));
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn from_raw_ptr_with_wiping() {
        let mut source_data = b"secret_data".to_vec();
        let ptr = source_data.as_mut_ptr();
        let len = source_data.len();

        let h = unsafe { HeapStr::<32>::from_raw_parts_mut(ptr, len) }.unwrap();
        assert_eq!(h.as_str(), "secret_data");

        assert_eq!(source_data, vec![0u8; 11]);
    }

    #[test]
    fn test_capacity_exceeded_fail_fast() {
        let res: Result<StackStr<4>, _> = "hello".try_into();
        assert!(res.is_err());
    }

    #[test]
    fn test_invalid_utf8_wiping() {
        let mut bad_utf8 = [0, 159, 146, 150];
        let res = StackStr::<16>::try_from(bad_utf8.as_mut_slice());
        assert!(res.is_err());
        assert_eq!(bad_utf8, [0u8; 4]);
    }
}
