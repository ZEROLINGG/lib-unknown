//! Unix 系统封装：各平台调用号/标志位（`syscall`）与 `dlopen`/`dlsym` 动态符号解析（`resolve`）。
//!
//! 需要启用 `"sys-unix"` 特性。
#[cfg(feature = "sys-syscall")]
pub mod syscall;

/// 通过 `dlopen/dlsym` 动态解析共享库符号。
///
/// 名称超长（≥256）、含内嵌 `\0` 或解析失败时返回 `None`，不缓存句柄。
///
/// # Examples
///
/// ```rust,no_run
/// use lib_unknown::sys::unix::resolve;
///
/// let f: Option<unsafe extern "C" fn() -> i32> = resolve("libc.so.6", "getpid");
/// let _ = f;
/// ```
#[inline(never)]
pub fn resolve<T: Sized + Copy>(dylib: &str, name: &str) -> Option<T> {
    use core::ffi::c_void;
    use core::mem;

    unsafe extern "C" {
        fn dlopen(filename: *const i8, flag: i32) -> *mut c_void;
        fn dlsym(handle: *mut c_void, symbol: *const i8) -> *mut c_void;
    }
    const RTLD_LAZY: i32 = 1;
    const MAX_LEN: usize = 256;

    if dylib.len() >= MAX_LEN || name.len() >= MAX_LEN {
        return None;
    }

    let mut dylib_buf = [0u8; MAX_LEN];
    let mut name_buf = [0u8; MAX_LEN];

    dylib_buf[..dylib.len()].copy_from_slice(dylib.as_bytes());
    name_buf[..name.len()].copy_from_slice(name.as_bytes());

    if dylib.as_bytes().contains(&0) || name.as_bytes().contains(&0) {
        return None;
    }

    // SAFETY: `dylib_buf` 以输入字节填充、剩余补 0，且已拒绝含内嵌 NUL 的输入，
    // 故指针指向合法 NUL 结尾字符串，调用期间有效；`dlopen` 为系统提供函数。
    let handle = unsafe { dlopen(dylib_buf.as_ptr() as *const i8, RTLD_LAZY) };
    if handle.is_null() {
        return None;
    }

    // SAFETY: `handle` 来自成功的 `dlopen`；`name_buf` 同理为合法 NUL 结尾字符串，
    // 调用期间有效。
    let sym = unsafe { dlsym(handle, name_buf.as_ptr() as *const i8) };
    if sym.is_null() {
        return None;
    }

    // SAFETY: 已检查 `sym` 非空；调用方以 `T: Sized + Copy` 声明目标签名，
    // 类型正确性由调用方保证（见函数文档）。
    Some(unsafe { mem::transmute_copy(&sym) })
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_() {
        let _x = c"2222222";
        let _y = r"2222222";
    }
}
