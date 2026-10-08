//! 定容安全内存类型：字节容器（`bytes`）、UTF-8 字符串容器（`str`）与 C 字符串容器（`cstr`），搬运/销毁时清零残留。
//!
//! 需要启用 `"types"`（或其子特性 `types-bytes` / `types-str` / `types-cstr`）特性。
#[cfg(feature = "types-bytes")]
pub mod bytes;
#[cfg(feature = "types-cstr")]
pub mod cstr;
#[cfg(feature = "types-str")]
pub mod str;
