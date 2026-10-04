//! 系统接口：裸系统调用（`syscall`）、Unix 动态符号解析（`unix`，类 Unix）、Windows DLL 解析（`win`，Windows）。
//!
//! 需要启用 `"sys"`（或其子特性 `sys-syscall` / `sys-unix` / `sys-win`）特性。
#[cfg(feature = "sys-syscall")]
pub mod syscall;
#[cfg(all(unix, feature = "sys-unix"))]
pub mod unix;
#[cfg(all(windows, feature = "sys-win"))]
pub mod win;
