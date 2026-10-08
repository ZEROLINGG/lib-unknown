//! 定容 C 字符串容器：栈上（`StackCStr`）与堆上（`HeapCStr`，需 `alloc`）实现，内部恒以 `\0` 结尾，搬运/销毁时清零残留。
//!
//! 与 [`crate::types::str`] 不同，本模块**允许非 UTF-8 字节**，仅校验 NUL 语义，
//! 可直接投喂 `sys_open` / `sys_execve` 等取 `core::ffi::CStr` 的调用。
//!
//! # 容量语义
//!
//! `N` **包含结尾 `\0`**（`N >= 1`）：有效载荷至多 `N - 1` 字节。
//! `len()` 返回不含 `\0` 的载荷长度。
//!
//! 需要启用 `"types-cstr"` 特性。
#![allow(unused_qualifications)]
#![allow(clippy::similar_names)]
#![allow(unused)]

#[cfg(feature = "alloc")]
extern crate alloc;

use super::bytes::{BytesError, volatile_zero};
use core::convert::TryFrom;
use core::ffi::CStr as StdCStr;

/// 安全 C 字符串的错误类型。
///
/// # Feature Requirement
///
/// 需要启用 `"types-cstr"` 特性。
///
/// # Examples
///
/// ```rust
/// use core::convert::TryInto;
/// use lib_unknown::types::cstr::{CStr, StackCStr};
///
/// let s: StackCStr<4> = "hi".try_into().unwrap();
/// assert_eq!(s.len(), 2);
/// let res: Result<StackCStr<4>, _> = "toolong".try_into();
/// assert!(res.is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CStrError {
    /// 长度超出容器容量（含结尾 `\0` 的总需求超出 `N`）。
    CapacityExceeded(BytesError),
    /// 载荷内部含 NUL，载荷为 NUL 在载荷内的字节下标。
    InteriorNul(usize),
    /// 输入中找不到结尾 NUL（源仍会被清零）。
    MissingNul,
    /// 收到空指针。
    NullPointer,
}

impl core::fmt::Display for CStrError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::CapacityExceeded(e) => write!(f, "capacity exceeded: {e}"),
            Self::InteriorNul(pos) => write!(f, "interior NUL at payload index {pos}"),
            Self::MissingNul => write!(f, "missing terminating NUL"),
            Self::NullPointer => write!(f, "received null pointer"),
        }
    }
}

impl core::error::Error for CStrError {}

#[inline(always)]
fn safe_parse_cstr_and_wipe<F>(buffer: &mut [u8], mut push_fn: F) -> Result<(), CStrError>
where
    F: FnMut(&[u8]) -> Result<(), CStrError>,
{
    let res = match buffer.iter().position(|&b| b == 0) {
        None => Err(CStrError::MissingNul),
        Some(pos) => push_fn(&buffer[..pos]),
    };
    volatile_zero(buffer);
    // SAFETY 交给 push_fn 的借用在 wipe 前结束；此处仅对源做易失清零。
    res
}

/// 安全 C 字符串的统一接口：只读视图，内部恒为合法 `CStr`（以 `\0` 结尾、无内部 NUL）。
///
/// 注意：本 trait 名为 `CStr`，与 `core::ffi::CStr` 同名。取值时用
/// [`CStr::as_cstr`] 拿到底层 `&core::ffi::CStr`。
///
/// # Feature Requirement
///
/// 需要启用 `"types-cstr"` 特性。
///
/// # Examples
///
/// ```rust
/// use core::convert::TryInto;
/// use lib_unknown::types::cstr::{CStr, StackCStr};
///
/// let s: StackCStr<8> = "hi".try_into().unwrap();
/// assert_eq!(s.as_cstr().to_bytes(), b"hi");
/// assert_eq!(s.len(), 2);
/// ```
pub trait CStr:
    ::core::ops::Deref<Target = StdCStr> + ::core::fmt::Display + ::core::fmt::Debug + Send + Sync
{
    /// 以 `&core::ffi::CStr` 形式查看内容，可直接传给 `sys_open` 等。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-cstr"` 特性。
    fn as_cstr(&self) -> &StdCStr;

    /// 以字节切片查看载荷（不含结尾 `\0`），允许非 UTF-8。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-cstr"` 特性。
    #[inline(always)]
    fn as_bytes(&self) -> &[u8] {
        self.as_cstr().to_bytes()
    }

    /// 以字节切片查看含结尾 `\0` 的完整存储。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-cstr"` 特性。
    #[inline(always)]
    fn as_bytes_with_nul(&self) -> &[u8] {
        self.as_cstr().to_bytes_with_nul()
    }

    /// 返回载荷的原始指针，指向容器内部存储（等价于 `as_cstr().as_ptr()`）。
    ///
    /// 返回的指针仅在 `self` 存活且未被搬运/修改期间有效；`self` 移动、`Drop`
    /// 或任何 `&mut self` 操作后不得再使用旧指针。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-cstr"` 特性。
    #[inline(always)]
    fn as_ptr(&self) -> *const core::ffi::c_char {
        self.as_cstr().as_ptr()
    }

    /// 类型擦除的堆克隆，用于 `Box<dyn CStr>` 容器。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-cstr"` 与 `"alloc"` 特性。
    #[cfg(feature = "alloc")]
    fn dyn_clone(&self) -> alloc::boxed::Box<dyn CStr>;

    /// 返回载荷的字节长度（不含结尾 `\0`）。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-cstr"` 特性。
    #[inline(always)]
    fn len(&self) -> usize {
        self.as_cstr().to_bytes().len()
    }

    /// 载荷是否为空。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-cstr"` 特性。
    #[inline(always)]
    fn is_empty(&self) -> bool {
        self.as_cstr().to_bytes().is_empty()
    }
}

/// 栈上定容安全 C 字符串，`Drop` 时自动清零，适用于短路径、argv 等小敏感文本。
///
/// `N` 包含结尾 `\0`，`N >= 1`。
///
/// # Feature Requirement
///
/// 需要启用 `"types-cstr"` 特性。
///
/// # Examples
///
/// ```rust
/// use core::convert::TryInto;
/// use lib_unknown::types::cstr::StackCStr;
///
/// let s: StackCStr<8> = "hi".try_into().unwrap();
/// assert_eq!(&*s, c"hi");
/// ```
pub struct StackCStr<const N: usize> {
    len: usize,
    data: [u8; N],
}

/// 堆上定容安全 C 字符串，语义与 [`StackCStr`] 一致，适用于较长的敏感文本。
///
/// `N` 包含结尾 `\0`，`N >= 1`。
///
/// # Feature Requirement
///
/// 需要启用 `"types-cstr"` 与 `"alloc"` 特性。
///
/// # Examples
///
/// ```rust
/// use core::convert::TryInto;
/// use lib_unknown::types::cstr::HeapCStr;
///
/// let s: HeapCStr<16> = "hi".try_into().unwrap();
/// assert_eq!(&*s, c"hi");
/// ```
#[cfg(feature = "alloc")]
pub struct HeapCStr<const N: usize> {
    len: usize,
    data: alloc::boxed::Box<[u8; N]>,
}

impl<const N: usize> StackCStr<N> {
    /// 容器总容量（含结尾 `\0`）。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-cstr"` 特性。
    pub const CAPACITY: usize = N;

    /// 创建空 C 字符串（仅含结尾 `\0`）。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-cstr"` 特性。
    ///
    /// # Panics
    ///
    /// - 当 `N == 0` 时 panic：连结尾 `\0` 都放不下。
    #[inline(always)]
    pub const fn new() -> Self {
        assert!(N > 0, "StackCStr capacity N must include the NUL (>= 1)");
        Self {
            len: 0,
            data: [0u8; N],
        }
    }
}

#[cfg(feature = "alloc")]
impl<const N: usize> HeapCStr<N> {
    /// 容器总容量（含结尾 `\0`）。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-cstr"` 与 `"alloc"` 特性。
    pub const CAPACITY: usize = N;

    /// 创建空 C 字符串（仅含结尾 `\0`）。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-cstr"` 与 `"alloc"` 特性。
    ///
    /// # Panics
    ///
    /// - 当 `N == 0` 时 panic：连结尾 `\0` 都放不下。
    #[inline(always)]
    pub fn new() -> Self {
        assert!(N > 0, "HeapCStr capacity N must include the NUL (>= 1)");
        // SAFETY: `[u8; N]` 全零为合法值（首字节 `\0` 即空 C 串），`assume_init` 安全。
        let data = unsafe { alloc::boxed::Box::<[u8; N]>::new_zeroed().assume_init() };
        Self { len: 0, data }
    }
}

macro_rules! impl_secure_cstr_base {
    ($name:ident) => {
        impl<const N: usize> $name<N> {
            #[inline(always)]
            fn try_push_bytes(&mut self, payload: &[u8]) -> Result<(), CStrError> {
                if let Some(pos) = payload.iter().position(|&b| b == 0) {
                    return Err(CStrError::InteriorNul(pos));
                }
                let required = self
                    .len
                    .checked_add(payload.len())
                    .and_then(|v| v.checked_add(1))
                    .ok_or(CStrError::CapacityExceeded(BytesError::CapacityExceeded {
                        requested: usize::MAX,
                        max: N,
                    }))?;
                if required > N {
                    return Err(CStrError::CapacityExceeded(BytesError::CapacityExceeded {
                        requested: required,
                        max: N,
                    }));
                }
                let cur = self.len;
                self.data[cur..cur + payload.len()].copy_from_slice(payload);
                self.data[cur + payload.len()] = 0;
                self.len = cur + payload.len();
                Ok(())
            }

            /// 从原始可变指针拷贝并擦除源缓冲构造安全 C 字符串。
            ///
            /// 在 `len` 范围内找首个 `\0` 截断为载荷；找不到则报
            /// [`CStrError::MissingNul`]。无论成败都清零源 `len` 字节。
            ///
            /// # Safety
            ///
            /// 调用此函数必须保证：
            /// 1. `ptr` 非空且在 `len` 范围内可读写、正确对齐；
            /// 2. 调用期间该内存不被其他线程并发访问。
            ///
            /// # Errors
            ///
            /// - `ptr` 为空时返回 [`CStrError::NullPointer`]。
            /// - `len` 内无 `\0` 时返回 [`CStrError::MissingNul`]。
            /// - 载荷（含结尾 `\0`）超出 `N` 时返回 [`CStrError::CapacityExceeded`]。
            pub unsafe fn from_raw_parts_mut(ptr: *mut u8, len: usize) -> Result<Self, CStrError> {
                if ptr.is_null() {
                    return Err(CStrError::NullPointer);
                }
                if len == 0 {
                    return Err(CStrError::MissingNul);
                }

                // SAFETY: `ptr` 非空已检查；有效性、对齐与独占性由本函数的 `# Safety` 约定保证。
                let slice = unsafe { ::core::slice::from_raw_parts_mut(ptr, len) };

                let mut out = Self::new();
                let pos = slice.iter().position(|&b| b == 0);
                let res: Result<(), CStrError> = match pos {
                    None => Err(CStrError::MissingNul),
                    Some(end) => out.try_push_bytes(&slice[..end]),
                };
                volatile_zero(slice);
                res?;
                Ok(out)
            }

            /// 将内容搬运到容量为 `M` 的同类 C 字符串中。
            ///
            /// 本方法按值取走 `self`，原容器在返回前（无论成败）被清零。
            ///
            /// # Feature Requirement
            ///
            /// 需要启用 `"types-cstr"` 特性（`HeapCStr` 相关还需 `"alloc"` 特性）。
            ///
            /// # Examples
            ///
            /// ```rust
            /// use core::convert::TryInto;
            /// use lib_unknown::types::cstr::StackCStr;
            ///
            /// let s: StackCStr<4> = "hi".try_into().unwrap();
            /// let c = s.try_grow::<8>().unwrap();
            /// assert_eq!(&*c, c"hi");
            /// ```
            ///
            /// # Errors
            ///
            /// - 当已有长度（含 `\0`）超出 `M` 时返回 [`CStrError::CapacityExceeded`]。
            ///
            /// # Panics
            ///
            /// - 内部 `expect("Capacity already checked")`：前置长度检查保证不触发，仅防御性保留。
            #[inline]
            pub fn try_grow<const M: usize>(self) -> Result<$name<M>, CStrError> {
                if self.len + 1 > M {
                    return Err(CStrError::CapacityExceeded(BytesError::CapacityExceeded {
                        requested: self.len + 1,
                        max: M,
                    }));
                }
                let mut larger = $name::<M>::new();
                larger
                    .try_push_bytes(self.as_bytes())
                    .expect("Capacity already checked");
                Ok(larger)
            }

            /// 将自身与 `suffix` 拼接为容量 `M` 的新 C 字符串。
            ///
            /// `suffix` 为不含 `\0` 的载荷字节（允许非 UTF-8）；含 `\0` 时报
            /// [`CStrError::InteriorNul`]，其下标为拼接后载荷中的绝对下标。
            /// 本方法按值取走 `self`，原容器在返回前（无论成败）被清零。
            ///
            /// # Feature Requirement
            ///
            /// 需要启用 `"types-cstr"` 特性（`HeapCStr` 相关还需 `"alloc"` 特性）。
            ///
            /// # Examples
            ///
            /// ```rust
            /// use core::convert::TryInto;
            /// use lib_unknown::types::cstr::StackCStr;
            ///
            /// let s: StackCStr<4> = "hi".try_into().unwrap();
            /// let c = s.push_bytes_into::<8>(b"!").unwrap();
            /// assert_eq!(&*c, c"hi!");
            /// ```
            ///
            /// # Errors
            ///
            /// - 当拼接后长度（含 `\0`）超出 `M` 时返回 [`CStrError::CapacityExceeded`]。
            /// - 当 `suffix` 含 `\0` 时返回 [`CStrError::InteriorNul`]。
            ///
            /// # Panics
            ///
            /// - 两处内部 `expect("Capacity already checked")`：前置长度检查保证不触发，仅防御性保留。
            #[inline]
            pub fn push_bytes_into<const M: usize>(
                self,
                suffix: &[u8],
            ) -> Result<$name<M>, CStrError> {
                if let Some(rel) = suffix.iter().position(|&b| b == 0) {
                    return Err(CStrError::InteriorNul(self.len + rel));
                }
                let required = self
                    .len
                    .checked_add(suffix.len())
                    .and_then(|v| v.checked_add(1))
                    .ok_or(CStrError::CapacityExceeded(BytesError::CapacityExceeded {
                        requested: usize::MAX,
                        max: M,
                    }))?;
                if required > M {
                    return Err(CStrError::CapacityExceeded(BytesError::CapacityExceeded {
                        requested: required,
                        max: M,
                    }));
                }
                let required = self.len + suffix.len() + 1;
                if required > M {
                    return Err(CStrError::CapacityExceeded(BytesError::CapacityExceeded {
                        requested: required,
                        max: M,
                    }));
                }
                let mut larger = $name::<M>::new();
                larger
                    .try_push_bytes(self.as_bytes())
                    .expect("Capacity already checked");
                larger
                    .try_push_bytes(suffix)
                    .expect("Capacity already checked");
                Ok(larger)
            }

            /// 将自身与 `rhs` 拼接为容量 `M` 的新 C 字符串（`rhs` 为任意可借用为字节切片的类型）。
            ///
            /// 本方法按值取走 `self`，原容器在返回前（无论成败）被清零。
            ///
            /// # Feature Requirement
            ///
            /// 需要启用 `"types-cstr"` 特性（`HeapCStr` 相关还需 `"alloc"` 特性）。
            ///
            /// # Examples
            ///
            /// ```rust
            /// use core::convert::TryInto;
            /// use lib_unknown::types::cstr::StackCStr;
            ///
            /// let s: StackCStr<4> = "hi".try_into().unwrap();
            /// let c = s.concat_into::<8>(b"!").unwrap();
            /// assert_eq!(&*c, c"hi!");
            /// ```
            ///
            /// # Errors
            ///
            /// - 当拼接后长度（含 `\0`）超出 `M` 时返回 [`CStrError::CapacityExceeded`]。
            /// - 当 `rhs` 含 `\0` 时返回 [`CStrError::InteriorNul`]，其下标为拼接后载荷中的绝对下标。
            ///
            /// # Panics
            ///
            /// - 内部 `expect("Capacity already checked")`：前置长度检查保证不触发，仅防御性保留。
            pub fn concat_into<const M: usize>(
                self,
                rhs: impl ::core::convert::AsRef<[u8]>,
            ) -> Result<$name<M>, CStrError> {
                self.push_bytes_into(rhs.as_ref())
            }
        }

        impl<const N: usize> ::core::default::Default for $name<N> {
            #[inline(always)]
            fn default() -> Self {
                Self::new()
            }
        }

        impl<const N: usize> ::core::ops::Drop for $name<N> {
            #[inline(always)]
            fn drop(&mut self) {
                volatile_zero(&mut self.data[..]);
            }
        }

        impl<const N: usize> CStr for $name<N> {
            #[inline(always)]
            fn as_cstr(&self) -> &StdCStr {
                // SAFETY: 类型不变式保证 `data[..len]` 无内部 NUL 且 `data[len] == 0`；
                // 所有写入经 `try_push_bytes` 校验，`new` 置首字节为 0。
                unsafe { StdCStr::from_bytes_with_nul_unchecked(&self.data[..self.len + 1]) }
            }

            #[cfg(feature = "alloc")]
            #[inline(always)]
            fn dyn_clone(&self) -> alloc::boxed::Box<dyn CStr> {
                alloc::boxed::Box::new(self.clone())
            }
        }

        impl<const N: usize> ::core::clone::Clone for $name<N> {
            #[inline(always)]
            fn clone(&self) -> Self {
                let mut out = Self::new();
                let _ = out.try_push_bytes(self.as_bytes());
                out
            }
        }
    };
}

macro_rules! impl_try_from_traits {
    ($name:ident) => {
        impl<const N: usize> TryFrom<&StdCStr> for $name<N> {
            type Error = CStrError;
            #[inline(always)]
            fn try_from(s: &StdCStr) -> Result<Self, Self::Error> {
                let mut out = Self::new();
                out.try_push_bytes(s.to_bytes())?;
                Ok(out)
            }
        }

        impl<const N: usize> TryFrom<&str> for $name<N> {
            type Error = CStrError;
            #[inline(always)]
            fn try_from(s: &str) -> Result<Self, Self::Error> {
                let mut out = Self::new();
                out.try_push_bytes(s.as_bytes())?;
                Ok(out)
            }
        }

        impl<const N: usize> TryFrom<&[u8]> for $name<N> {
            type Error = CStrError;
            fn try_from(s: &[u8]) -> Result<Self, Self::Error> {
                match s.iter().position(|&b| b == 0) {
                    None => Err(CStrError::MissingNul),
                    Some(pos) => {
                        let mut out = Self::new();
                        out.try_push_bytes(&s[..pos])?;
                        Ok(out)
                    }
                }
            }
        }

        impl<const N: usize> ::core::str::FromStr for $name<N> {
            type Err = CStrError;
            #[inline(always)]
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::try_from(s)
            }
        }

        impl<const N: usize> TryFrom<&mut [u8; N]> for $name<N> {
            type Error = CStrError;
            fn try_from(arr: &mut [u8; N]) -> Result<Self, Self::Error> {
                let mut out = Self::new();
                safe_parse_cstr_and_wipe(&mut arr[..], |p| out.try_push_bytes(p))?;
                Ok(out)
            }
        }

        impl<const N: usize> TryFrom<&mut [u8]> for $name<N> {
            type Error = CStrError;
            fn try_from(arr: &mut [u8]) -> Result<Self, Self::Error> {
                let mut out = Self::new();
                safe_parse_cstr_and_wipe(arr, |p| out.try_push_bytes(p))?;
                Ok(out)
            }
        }

        #[cfg(feature = "alloc")]
        impl<const N: usize> TryFrom<alloc::boxed::Box<[u8; N]>> for $name<N> {
            type Error = CStrError;
            fn try_from(mut b: alloc::boxed::Box<[u8; N]>) -> Result<Self, Self::Error> {
                let mut out = Self::new();
                safe_parse_cstr_and_wipe(&mut *b, |p| out.try_push_bytes(p))?;
                Ok(out)
            }
        }

        #[cfg(feature = "alloc")]
        impl<const N: usize> TryFrom<alloc::vec::Vec<u8>> for $name<N> {
            type Error = CStrError;
            fn try_from(mut v: alloc::vec::Vec<u8>) -> Result<Self, Self::Error> {
                let mut out = Self::new();
                safe_parse_cstr_and_wipe(v.as_mut_slice(), |p| out.try_push_bytes(p))?;
                Ok(out)
            }
        }

        #[cfg(feature = "alloc")]
        impl<const N: usize> TryFrom<alloc::string::String> for $name<N> {
            type Error = CStrError;
            #[inline(always)]
            fn try_from(s: alloc::string::String) -> Result<Self, Self::Error> {
                Self::try_from(s.into_bytes())
            }
        }
    };
}

#[inline(always)]
fn fmt_cstr_lossy(f: &mut core::fmt::Formatter<'_>, mut rest: &[u8]) -> core::fmt::Result {
    loop {
        match core::str::from_utf8(rest) {
            Ok(valid) => return f.write_str(valid),
            Err(e) => {
                let valid = e.valid_up_to();
                if valid > 0 {
                    // SAFETY: `valid_up_to` 前缀经校验为合法 UTF-8。
                    f.write_str(unsafe { core::str::from_utf8_unchecked(&rest[..valid]) })?;
                }
                f.write_str("\u{FFFD}")?;
                rest = &rest[valid + e.error_len().unwrap_or(1)..];
                if rest.is_empty() {
                    return Ok(());
                }
            }
        }
    }
}

macro_rules! __impl_common_cstr_traits {
    ($name:ident) => {
        impl<const N: usize> ::core::ops::Deref for $name<N> {
            type Target = StdCStr;
            #[inline(always)]
            fn deref(&self) -> &Self::Target {
                self.as_cstr()
            }
        }
        impl<const N: usize> ::core::fmt::Display for $name<N> {
            #[inline(always)]
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                fmt_cstr_lossy(f, self.as_bytes())
            }
        }
        impl<const N: usize> ::core::fmt::Debug for $name<N> {
            #[inline(always)]
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                ::core::fmt::Debug::fmt(self.as_cstr(), f)
            }
        }
        impl<const N: usize> ::core::convert::AsRef<StdCStr> for $name<N> {
            #[inline(always)]
            fn as_ref(&self) -> &StdCStr {
                self.as_cstr()
            }
        }
        impl<const N: usize> ::core::convert::AsRef<[u8]> for $name<N> {
            #[inline(always)]
            fn as_ref(&self) -> &[u8] {
                self.as_bytes()
            }
        }
        impl<const N: usize> ::core::cmp::PartialEq<StdCStr> for $name<N> {
            #[inline(always)]
            fn eq(&self, other: &StdCStr) -> bool {
                self.as_cstr() == other
            }
        }
        impl<'a, const N: usize> ::core::cmp::PartialEq<&'a StdCStr> for $name<N> {
            #[inline(always)]
            fn eq(&self, other: &&'a StdCStr) -> bool {
                self.as_cstr() == *other
            }
        }
        impl<const N: usize> ::core::cmp::PartialEq<str> for $name<N> {
            #[inline(always)]
            fn eq(&self, other: &str) -> bool {
                self.as_bytes() == other.as_bytes()
            }
        }
        impl<'a, const N: usize> ::core::cmp::PartialEq<&'a str> for $name<N> {
            #[inline(always)]
            fn eq(&self, other: &&'a str) -> bool {
                self.as_bytes() == other.as_bytes()
            }
        }
        impl<const N: usize, const M: usize> ::core::cmp::PartialEq<$name<M>> for $name<N> {
            #[inline(always)]
            fn eq(&self, other: &$name<M>) -> bool {
                self.as_cstr() == other.as_cstr()
            }
        }
        #[cfg(feature = "alloc")]
        impl<const N: usize> ::core::cmp::PartialEq<alloc::string::String> for $name<N> {
            #[inline(always)]
            fn eq(&self, other: &alloc::string::String) -> bool {
                self.as_bytes() == other.as_bytes()
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
                self.as_cstr().cmp(other.as_cstr())
            }
        }
        impl<const N: usize> ::core::hash::Hash for $name<N> {
            #[inline(always)]
            fn hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
                // 必须与 `core::ffi::CStr` 自身的 `Hash`（派生自含结尾 `\0` 的内部切片）一致，
                // 否则违反 `Borrow<StdCStr>` 的哈希契约（以 `&CStr` 查 `HashMap` 会失效）。
                self.as_cstr().hash(state)
            }
        }
        impl<const N: usize> ::core::borrow::Borrow<StdCStr> for $name<N> {
            #[inline(always)]
            fn borrow(&self) -> &StdCStr {
                self.as_cstr()
            }
        }
    };
}

impl_secure_cstr_base!(StackCStr);
impl_try_from_traits!(StackCStr);
__impl_common_cstr_traits!(StackCStr);

#[cfg(feature = "alloc")]
impl_secure_cstr_base!(HeapCStr);
#[cfg(feature = "alloc")]
impl_try_from_traits!(HeapCStr);
#[cfg(feature = "alloc")]
__impl_common_cstr_traits!(HeapCStr);

#[cfg(feature = "alloc")]
impl<const N: usize> StackCStr<N> {
    /// 将栈 C 字符串搬运为容量 `M` 的堆 C 字符串。
    ///
    /// 本方法按值取走 `self`，原容器在返回前（无论成败）被清零。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"types-cstr"` 与 `"alloc"` 特性。
    ///
    /// # Examples
    ///
    /// ```rust
    /// use core::convert::TryInto;
    /// use lib_unknown::types::cstr::StackCStr;
    ///
    /// let s: StackCStr<4> = "hi".try_into().unwrap();
    /// let h = s.into_heap::<8>().unwrap();
    /// assert_eq!(&*h, c"hi");
    /// ```
    ///
    /// # Errors
    ///
    /// - 当已有长度（含 `\0`）超出 `M` 时返回 [`CStrError::CapacityExceeded`]。
    ///
    /// # Panics
    ///
    /// - 内部 `expect("Capacity checked")`：前置长度检查保证不触发，仅防御性保留。
    #[inline]
    pub fn into_heap<const M: usize>(self) -> Result<HeapCStr<M>, CStrError> {
        if self.len + 1 > M {
            return Err(CStrError::CapacityExceeded(BytesError::CapacityExceeded {
                requested: self.len + 1,
                max: M,
            }));
        }
        let mut heap = HeapCStr::<M>::new();
        heap.try_push_bytes(self.as_bytes())
            .expect("Capacity checked");
        Ok(heap)
    }
}

#[cfg(feature = "alloc")]
impl<const N: usize, const M: usize> ::core::cmp::PartialEq<StackCStr<M>> for HeapCStr<N> {
    #[inline(always)]
    fn eq(&self, other: &StackCStr<M>) -> bool {
        self.as_cstr() == other.as_cstr()
    }
}
#[cfg(feature = "alloc")]
impl<const N: usize, const M: usize> ::core::cmp::PartialEq<HeapCStr<N>> for StackCStr<M> {
    #[inline(always)]
    fn eq(&self, other: &HeapCStr<N>) -> bool {
        self.as_cstr() == other.as_cstr()
    }
}

#[cfg(feature = "alloc")]
impl ::core::clone::Clone for alloc::boxed::Box<dyn CStr> {
    #[inline(always)]
    fn clone(&self) -> Self {
        self.dyn_clone()
    }
}
#[cfg(feature = "alloc")]
impl ::core::cmp::PartialEq for alloc::boxed::Box<dyn CStr> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.as_cstr() == other.as_cstr()
    }
}
#[cfg(feature = "alloc")]
impl ::core::cmp::Eq for alloc::boxed::Box<dyn CStr> {}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    #[cfg(feature = "alloc")]
    use alloc::{boxed::Box, vec, vec::Vec};
    use core::convert::TryInto;

    #[test]
    fn test_stack_from_str_literal() {
        let s: StackCStr<8> = "hello".try_into().unwrap();
        assert_eq!(&*s, c"hello");
        assert_eq!(s.len(), 5);
        assert_eq!(s.as_bytes_with_nul(), b"hello\0");
    }

    #[test]
    fn test_non_utf8_payload() {
        let s: StackCStr<8> = StackCStr::try_from([0xFFu8, 0xFE, 0].as_slice()).unwrap();
        assert_eq!(s.as_bytes(), &[0xFF, 0xFE]);
        // Display 走 lossy，不 panic 即可
        let _ = alloc_display(&s);
    }

    #[cfg(feature = "alloc")]
    fn alloc_display(s: &StackCStr<8>) -> alloc::string::String {
        use alloc::string::ToString;
        s.to_string()
    }

    #[cfg(not(feature = "alloc"))]
    fn alloc_display(s: &StackCStr<8>) -> usize {
        use core::fmt::Write;
        struct Counter(usize);
        impl Write for Counter {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                self.0 += s.len();
                Ok(())
            }
        }
        let mut c = Counter(0);
        let _ = write!(c, "{s}");
        c.0
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_heap_from_str_literal() {
        let h: HeapCStr<16> = "hello".try_into().unwrap();
        assert_eq!(&*h, c"hello");
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_vec_dyn_cstr() {
        let mut list: Vec<Box<dyn CStr>> = Vec::new();
        let h: HeapCStr<256> = "/tmp/secret".try_into().unwrap();
        list.push(Box::new(h));
        let s: StackCStr<16> = "/bin/sh".try_into().unwrap();
        list.push(Box::new(s));
        assert_eq!(list.len(), 2);
        assert!(list[1].as_bytes().starts_with(b"/bin"));
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn from_raw_ptr_with_wiping() {
        let mut source_data = b"/tmp/x\0".to_vec();
        let ptr = source_data.as_mut_ptr();
        let len = source_data.len();

        // SAFETY: `ptr` 指向 `source_data` 的独占可写借用派生的有效内存，长度 `len` 精确覆盖向量本体，测试内无并发访问。
        let h = unsafe { HeapCStr::<32>::from_raw_parts_mut(ptr, len) }.unwrap();
        assert_eq!(h.as_bytes(), b"/tmp/x");

        assert_eq!(source_data, vec![0u8; 7]);
    }

    #[test]
    fn test_capacity_exceeded_fail_fast() {
        // N 含 \0：容量 4 只能放 3 载荷字节
        let res: Result<StackCStr<4>, _> = "hello".try_into();
        assert!(res.is_err());
        let ok: Result<StackCStr<4>, _> = "hey".try_into();
        assert!(ok.is_ok());
    }

    #[test]
    fn test_interior_nul_rejected() {
        let res = StackCStr::<16>::try_from("a\0b");
        assert!(matches!(res, Err(CStrError::InteriorNul(1))));
    }

    #[test]
    fn test_missing_nul_wiping() {
        let mut no_nul = *b"abc";
        let res = StackCStr::<16>::try_from(no_nul.as_mut_slice());
        assert!(matches!(res, Err(CStrError::MissingNul)));
        assert_eq!(no_nul, [0u8; 3]);
    }

    #[test]
    fn test_truncates_at_first_nul_and_wipes() {
        let mut buf = *b"hi\0garbage";
        let s = StackCStr::<16>::try_from(buf.as_mut_slice()).unwrap();
        assert_eq!(s.as_bytes(), b"hi");
        assert_eq!(buf, [0u8; 10]);
    }

    #[test]
    fn test_capacity_one_holds_only_empty() {
        let e: StackCStr<1> = "".try_into().unwrap();
        assert!(e.is_empty());
        assert_eq!(e.as_bytes_with_nul(), b"\0");
        let res: Result<StackCStr<1>, _> = "a".try_into();
        assert!(matches!(res, Err(CStrError::CapacityExceeded(_))));
    }

    #[test]
    fn test_try_grow_and_shrink_fails() {
        let s: StackCStr<4> = "hi".try_into().unwrap();
        let big = s.try_grow::<8>().unwrap();
        assert_eq!(&*big, c"hi");
        let res = big.try_grow::<2>();
        assert!(matches!(res, Err(CStrError::CapacityExceeded(_))));
    }

    #[test]
    fn test_concat_into_overflow() {
        let s: StackCStr<4> = "hi".try_into().unwrap();
        let res = s.concat_into::<4>(b"!!");
        assert!(matches!(res, Err(CStrError::CapacityExceeded(_))));
    }

    #[test]
    fn test_display_lossy_replaces_invalid() {
        use core::fmt::Write;
        let s: StackCStr<8> = StackCStr::try_from([0xFFu8, 0xFE, 0].as_slice()).unwrap();
        struct Buf<'a>(&'a mut [u8; 16], usize);
        impl Write for Buf<'_> {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                let b = s.as_bytes();
                self.0[self.1..self.1 + b.len()].copy_from_slice(b);
                self.1 += b.len();
                Ok(())
            }
        }
        let mut storage = [0u8; 16];
        let written = {
            let mut buf = Buf(&mut storage, 0);
            write!(buf, "{s}").unwrap();
            buf.1
        };
        assert_eq!(&storage[..written], "��".as_bytes());
    }

    #[test]
    fn test_as_ptr_and_borrow_hash_consistent() {
        use core::borrow::Borrow;
        use core::ffi::CStr as StdCStr;
        use core::hash::{Hash, Hasher};
        struct Fnv(u64);
        impl Hasher for Fnv {
            fn write(&mut self, b: &[u8]) {
                for &x in b {
                    self.0 = self.0.wrapping_mul(0x100000001b3).wrapping_add(x as u64);
                }
            }
            fn finish(&self) -> u64 {
                self.0
            }
        }
        let s: StackCStr<8> = "hi".try_into().unwrap();
        assert!(!s.as_ptr().is_null());
        // SAFETY: `as_ptr` 指向 `s` 内部以 `\0` 结尾的有效存储，断言求值期间 `s` 存活且无修改。
        assert_eq!(unsafe { StdCStr::from_ptr(s.as_ptr()) }, c"hi");
        let borrowed: &StdCStr = s.borrow();
        assert_eq!(borrowed, c"hi");
        let mut h1 = Fnv(0xcbf29ce484222325);
        s.hash(&mut h1);
        let mut h2 = Fnv(0xcbf29ce484222325);
        borrowed.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }

    #[test]
    fn test_from_str_parse() {
        let s: StackCStr<8> = "hi".parse().unwrap();
        assert_eq!(&*s, c"hi");
        let res = "a\0b".parse::<StackCStr<8>>();
        assert!(matches!(res, Err(CStrError::InteriorNul(1))));
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_into_heap_roundtrip() {
        let s: StackCStr<4> = "hi".try_into().unwrap();
        let h = s.into_heap::<8>().unwrap();
        assert_eq!(&*h, c"hi");
        assert_eq!(h, HeapCStr::<8>::try_from("hi").unwrap());
    }
}
