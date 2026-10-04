//! Unix 裸调用的高层封装：各平台调用号（`constants`）、标志位（`flags`）、`ptrace` 请求号（`ptrace_req`）、统一错误（`SysResult` / `SysErr`）与常用调用封装（`sys_*`）。
//!
//! 需要启用 `"sys-unix"` 特性。

use core::error::Error;
use core::ffi::CStr;
use core::fmt::{Debug, Display, Formatter};

use crate::sys::syscall::*;
// 实现示例
// #[inline(always)]
// pub unsafe fn syscall0(n: usize) -> isize {
//     let ret: isize;
//
//     // =========================================================================
//     // Linux & Android
//     // =========================================================================
//
//     #[cfg(all(
//         any(target_os = "linux", target_os = "android"),
//         target_arch = "x86_64"
//     ))]
//     unsafe {
//         core::arch::asm!(
//         "syscall",
//         in("rax") n,
//         lateout("rax") ret,
//         lateout("rcx") _, // syscall 指令硬件语义：rcx 保存返回地址，被内核覆盖
//         lateout("r11") _, // syscall 指令硬件语义：r11 保存 RFLAGS，被内核覆盖
//         options(nostack)  // syscall 不触碰用户栈；内核可能修改 EFLAGS，故不加 preserves_flags
//         );
//     }
//
//     #[cfg(all(
//         any(target_os = "linux", target_os = "android"),
//         target_arch = "aarch64"
//     ))]
//     unsafe {
//         core::arch::asm!(
//         "svc #0",
//         in("x8") n,
//         lateout("x0") ret,
//         options(nostack) // Linux aarch64 syscall ABI 只保证修改 x0，其余寄存器保留
//         );
//     }
//
//     #[cfg(all(
//         any(target_os = "linux", target_os = "android"),
//         target_arch = "riscv64"
//     ))]
//     unsafe {
//         core::arch::asm!(
//         "ecall",
//         in("a7") n,
//         lateout("a0") ret,
//         options(nostack) // 同理，Linux riscv64 syscall 只保证修改 a0
//         );
//     }
//
//     #[cfg(all(any(target_os = "linux", target_os = "android"), target_arch = "x86"))]
//     unsafe {
//         core::arch::asm!(
//         "int 0x80",
//         in("eax") n,
//         lateout("eax") ret,
//         options(nostack)
//         );
//     }
//
//     // =========================================================================
//     // macOS (Darwin)
//     // =========================================================================
//
//     #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
//     unsafe {
//         let sys_num = n | 0x2000000;
//         core::arch::asm!(
//         "syscall",
//         "jnc 1f",
//         "neg rax",
//         "1:",
//         in("rax") sys_num,
//         lateout("rax") ret,
//         lateout("rcx") _,
//         lateout("r11") _,
//         options(nostack)
//         );
//     }
//
//     #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
//     unsafe {
//         core::arch::asm!(
//         "svc #0x80",
//         "b.cc 1f",
//         "neg x0, x0",
//         "1:",
//         in("x16") n,
//         lateout("x0") ret,
//         options(nostack)
//         );
//     }
//
//     // =========================================================================
//     // Windows (NT内核)
//     // =========================================================================
//
//     #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
//     unsafe {
//         core::arch::asm!(
//         "syscall",
//         in("rax") n,
//         lateout("rax") ret,
//         lateout("rcx") _,
//         lateout("r11") _,
//         lateout("r10") _,
//         lateout("rdx") _,
//         lateout("r8") _,
//         lateout("r9") _,
//         options(nostack)
//         );
//     }
//
//     #[cfg(all(target_os = "windows", target_arch = "x86"))]
//     unsafe {
//         core::arch::asm!(
//         "int 0x2e",
//         in("eax") n,
//         lateout("eax") ret,
//         lateout("ecx") _,
//         lateout("edx") _,
//         options(nostack)
//         );
//     }
//
//     #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
//     unsafe {
//         core::arch::asm!(
//         "svc #0",
//         in("x8") n,
//         lateout("x0") ret,
//         options(nostack)
//         );
//     }
//
//     unsupported_target!("syscall0");
//
//     ret
// }

// =========================================================================
// Linux x86_64
// =========================================================================
/// Linux x86_64 平台的裸系统调用号。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-unix"` 特性。
#[cfg(all(
    any(target_os = "linux", target_os = "android"),
    target_arch = "x86_64"
))]
pub mod constants {
    pub const READ: usize = 0;
    pub const WRITE: usize = 1;
    pub const OPEN: usize = 2;
    pub const CLOSE: usize = 3;
    pub const LSEEK: usize = 8;
    pub const MMAP: usize = 9;
    pub const MPROTECT: usize = 10;
    pub const MUNMAP: usize = 11;
    pub const BRK: usize = 12;
    pub const RT_SIGACTION: usize = 13;
    pub const RT_SIGPROCMASK: usize = 14;
    pub const IOCTL: usize = 16;
    pub const DUP: usize = 32;
    pub const DUP2: usize = 33;
    pub const NANOSLEEP: usize = 35;
    pub const GETPID: usize = 39;
    pub const SOCKET: usize = 41;
    pub const CONNECT: usize = 42;
    pub const ACCEPT: usize = 43;
    pub const SENDTO: usize = 44;
    pub const RECVFROM: usize = 45;
    pub const BIND: usize = 49;
    pub const LISTEN: usize = 50;
    pub const SETSOCKOPT: usize = 54;
    pub const GETSOCKOPT: usize = 55;
    pub const CLONE: usize = 56;
    pub const FORK: usize = 57;
    pub const EXECVE: usize = 59;
    pub const EXIT: usize = 60;
    pub const WAIT4: usize = 61;
    pub const KILL: usize = 62;
    pub const FCNTL: usize = 72;
    pub const FSYNC: usize = 74;
    pub const GETCWD: usize = 79;
    pub const CHDIR: usize = 80;
    pub const MKDIR: usize = 83;
    pub const UNLINK: usize = 87;
    pub const GETUID: usize = 102;
    pub const GETPPID: usize = 110;
    pub const PRCTL: usize = 157;
    pub const GETTID: usize = 186;
    pub const GETDENTS64: usize = 217;
    pub const CLOCK_GETTIME: usize = 228;
    pub const EPOLL_WAIT: usize = 232;
    pub const EPOLL_CTL: usize = 233;
    pub const OPENAT: usize = 257;
    pub const MKDIRAT: usize = 258;
    pub const NEWFSTATAT: usize = 262;
    pub const UNLINKAT: usize = 263;
    pub const RENAMEAT: usize = 264;
    pub const READLINKAT: usize = 267;
    pub const EPOLL_CREATE1: usize = 291;
    pub const DUP3: usize = 292;
    pub const PIPE2: usize = 293;
    pub const STATX: usize = 332;

    pub const PTRACE: usize = 101;
}

// =========================================================================
// Linux x86 (32-bit)
// =========================================================================
/// Linux x86（32 位）平台的裸系统调用号。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-unix"` 特性。
#[cfg(all(any(target_os = "linux", target_os = "android"), target_arch = "x86"))]
pub mod constants {
    pub const EXIT: usize = 1;
    pub const FORK: usize = 2;
    pub const READ: usize = 3;
    pub const WRITE: usize = 4;
    pub const OPEN: usize = 5;
    pub const CLOSE: usize = 6;
    pub const UNLINK: usize = 10;
    pub const EXECVE: usize = 11;
    pub const CHDIR: usize = 12;
    pub const LSEEK: usize = 19;
    pub const GETPID: usize = 20;
    pub const GETUID: usize = 24;
    pub const KILL: usize = 37;
    pub const MKDIR: usize = 39;
    pub const BRK: usize = 45;
    pub const IOCTL: usize = 54;
    pub const FCNTL: usize = 55;
    pub const DUP2: usize = 63;
    pub const GETPPID: usize = 64;
    pub const MUNMAP: usize = 91;
    pub const SOCKETCALL: usize = 102;
    pub const WAIT4: usize = 114;
    pub const FSYNC: usize = 118;
    pub const CLONE: usize = 120;
    pub const MPROTECT: usize = 125;
    pub const NANOSLEEP: usize = 162;
    pub const GETCWD: usize = 183;
    pub const MMAP2: usize = 192;
    pub const GETDENTS64: usize = 220;
    pub const GETTID: usize = 224;
    pub const EPOLL_CTL: usize = 255;
    pub const EPOLL_WAIT: usize = 256;
    pub const CLOCK_GETTIME: usize = 265;
    pub const OPENAT: usize = 295;
    pub const MKDIRAT: usize = 296;
    pub const NEWFSTATAT: usize = 300;
    pub const UNLINKAT: usize = 301;
    pub const RENAMEAT: usize = 302;
    pub const READLINKAT: usize = 305;
    pub const EPOLL_CREATE1: usize = 329;
    pub const DUP3: usize = 330;
    pub const PIPE2: usize = 331;
    pub const SOCKET: usize = 359;
    pub const BIND: usize = 361;
    pub const CONNECT: usize = 362;
    pub const LISTEN: usize = 363;
    pub const ACCEPT: usize = 364;
    pub const GETSOCKOPT: usize = 365;
    pub const SETSOCKOPT: usize = 366;
    pub const STATX: usize = 383;
    pub const PTRACE: usize = 26;
}

// =========================================================================
// Linux aarch64 / riscv64 (asm-generic)
// =========================================================================
/// Linux aarch64 / riscv64 平台的裸系统调用号（asm-generic）。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-unix"` 特性。
#[cfg(all(
    any(target_os = "linux", target_os = "android"),
    any(target_arch = "aarch64", target_arch = "riscv64")
))]
pub mod constants {
    pub const GETCWD: usize = 17;
    pub const EPOLL_CREATE1: usize = 20;
    pub const EPOLL_CTL: usize = 21;
    pub const DUP3: usize = 24;
    pub const FCNTL: usize = 25;
    pub const IOCTL: usize = 29;
    pub const MKDIRAT: usize = 34;
    pub const UNLINKAT: usize = 35;
    pub const RENAMEAT: usize = 38;
    pub const CHDIR: usize = 49;
    pub const OPENAT: usize = 56;
    pub const CLOSE: usize = 57;
    pub const PIPE2: usize = 59;
    pub const GETDENTS64: usize = 61;
    pub const LSEEK: usize = 62;
    pub const READ: usize = 63;
    pub const WRITE: usize = 64;
    pub const READLINKAT: usize = 78;
    pub const NEWFSTATAT: usize = 79;
    pub const FSYNC: usize = 82;
    pub const EXIT: usize = 93;
    pub const CLOCK_GETTIME: usize = 113;
    pub const KILL: usize = 129;
    pub const RT_SIGACTION: usize = 134;
    pub const RT_SIGPROCMASK: usize = 135;
    pub const PRCTL: usize = 167;
    pub const GETPID: usize = 172;
    pub const GETPPID: usize = 173;
    pub const GETUID: usize = 174;
    pub const GETTID: usize = 178;
    pub const SOCKET: usize = 198;
    pub const BIND: usize = 200;
    pub const LISTEN: usize = 201;
    pub const ACCEPT: usize = 202;
    pub const CONNECT: usize = 203;
    pub const GETSOCKOPT: usize = 209;
    pub const SETSOCKOPT: usize = 208;
    pub const BRK: usize = 214;
    pub const MUNMAP: usize = 215;
    pub const CLONE: usize = 220;
    pub const EXECVE: usize = 221;
    pub const MMAP: usize = 222;
    pub const MPROTECT: usize = 226;
    pub const WAIT4: usize = 260;
    pub const STATX: usize = 291;
    pub const PTRACE: usize = 117;
}

// =========================================================================
// macOS (Darwin) XNU BSD Syscalls
// =========================================================================
/// macOS（Darwin XNU）平台的 BSD 系统调用号。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-unix"` 特性。
#[cfg(target_os = "macos")]
pub mod constants {
    pub const EXIT: usize = 1;
    pub const FORK: usize = 2;
    pub const READ: usize = 3;
    pub const WRITE: usize = 4;
    pub const OPEN: usize = 5;
    pub const CLOSE: usize = 6;
    pub const WAIT4: usize = 7;
    pub const UNLINK: usize = 10;
    pub const CHDIR: usize = 12;
    pub const GETPID: usize = 20;
    pub const GETUID: usize = 24;
    pub const KILL: usize = 37;
    pub const GETPPID: usize = 39;
    pub const SIGACTION: usize = 46;
    pub const IOCTL: usize = 54;
    pub const EXECVE: usize = 59;
    pub const MUNMAP: usize = 73;
    pub const MPROTECT: usize = 74;
    pub const DUP2: usize = 90;
    pub const FCNTL: usize = 92;
    pub const FSYNC: usize = 95;
    pub const SOCKET: usize = 97;
    pub const CONNECT: usize = 98;
    pub const BIND: usize = 104;
    pub const SETSOCKOPT: usize = 105;
    pub const LISTEN: usize = 106;
    pub const GETSOCKOPT: usize = 118;
    pub const MKDIR: usize = 136;
    pub const RMDIR: usize = 137;
    pub const MMAP: usize = 197;
    pub const GETCWD: usize = 277;
    pub const KQUEUE: usize = 362;
    pub const KEVENT: usize = 363;
    pub const KEVENT64: usize = 369;
    pub const THREAD_SELFID: usize = 372; // macOS 特有的获取 TID 方式
    pub const OPENAT: usize = 460;
    pub const RENAMEAT: usize = 464;
    pub const READLINKAT: usize = 467;
    pub const FSTATAT: usize = 469;
    pub const PTRACE: usize = 26;
}

// =========================================================================
// 标志位 (Flags) 针对非 macOS 平台
// =========================================================================
/// 非 macOS 平台的 open/mmap/socket 等标志位常量。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-unix"` 特性。
#[cfg(not(target_os = "macos"))]
pub mod flags {
    pub const O_RDONLY: usize = 0;
    pub const O_WRONLY: usize = 1;
    pub const O_RDWR: usize = 2;
    pub const O_CREAT: usize = 0o100;
    pub const O_NONBLOCK: usize = 0o4000;
    pub const O_CLOEXEC: usize = 0o2000000;
    pub const AT_FDCWD: usize = -100_isize as usize;

    pub const PROT_NONE: usize = 0;
    pub const PROT_READ: usize = 1;
    pub const PROT_WRITE: usize = 2;
    pub const PROT_EXEC: usize = 4;

    pub const MAP_SHARED: usize = 0x01;
    pub const MAP_PRIVATE: usize = 0x02;
    pub const MAP_ANONYMOUS: usize = 0x20;

    pub const CLOCK_REALTIME: usize = 0;
    pub const CLOCK_MONOTONIC: usize = 1;

    pub const EPOLL_CTL_ADD: usize = 1;
    pub const EPOLL_CTL_DEL: usize = 2;
    pub const EPOLL_CTL_MOD: usize = 3;
    pub const EPOLLIN: u32 = 0x001;
    pub const EPOLLOUT: u32 = 0x004;
    pub const EPOLLET: u32 = 1 << 31;

    pub const AF_INET: usize = 2;
    pub const AF_INET6: usize = 10;
    pub const SOCK_STREAM: usize = 1;
    pub const SOCK_DGRAM: usize = 2;

    pub const SOL_SOCKET: usize = 1;
    pub const SO_REUSEADDR: usize = 2;
    pub const SO_KEEPALIVE: usize = 9;
    pub const MSG_DONTWAIT: usize = 0x40;
}

// =========================================================================
// 标志位 (Flags) 针对 macOS 平台
// =========================================================================
/// macOS 平台的 open/mmap/socket 等标志位常量。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-unix"` 特性。
#[cfg(target_os = "macos")]
pub mod flags {
    pub const O_RDONLY: usize = 0x0000;
    pub const O_WRONLY: usize = 0x0001;
    pub const O_RDWR: usize = 0x0002;
    pub const O_NONBLOCK: usize = 0x0004;
    pub const O_CREAT: usize = 0x0200;
    pub const O_CLOEXEC: usize = 0x1000000;
    pub const AT_FDCWD: usize = -2_isize as usize;

    pub const PROT_NONE: usize = 0;
    pub const PROT_READ: usize = 1;
    pub const PROT_WRITE: usize = 2;
    pub const PROT_EXEC: usize = 4;

    pub const MAP_SHARED: usize = 1;
    pub const MAP_PRIVATE: usize = 2;
    pub const MAP_ANONYMOUS: usize = 0x1000;

    pub const AF_INET: usize = 2;
    pub const AF_INET6: usize = 30;
    pub const SOCK_STREAM: usize = 1;
    pub const SOCK_DGRAM: usize = 2;

    pub const SOL_SOCKET: usize = 0xffff;
    pub const SO_REUSEADDR: usize = 0x0004;
    pub const SO_KEEPALIVE: usize = 0x0008;

    pub const EV_ADD: u16 = 0x0001;
    pub const EV_DELETE: u16 = 0x0002;
    pub const EV_CLEAR: u16 = 0x0020;
    pub const EVFILT_READ: i16 = -1;
    pub const EVFILT_WRITE: i16 = -2;
}

/// 非 macOS 平台的 `ptrace` 请求号常量。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-unix"` 特性。
#[cfg(not(target_os = "macos"))]
pub mod ptrace_req {
    pub const PTRACE_TRACEME: usize = 0;
    pub const PTRACE_PEEKTEXT: usize = 1;
    pub const PTRACE_PEEKDATA: usize = 2;
    pub const PTRACE_PEEKUSER: usize = 3;
    pub const PTRACE_POKETEXT: usize = 4;
    pub const PTRACE_POKEDATA: usize = 5;
    pub const PTRACE_POKEUSER: usize = 6;
    pub const PTRACE_CONT: usize = 7;
    pub const PTRACE_KILL: usize = 8;
    pub const PTRACE_SINGLESTEP: usize = 9;
    pub const PTRACE_GETREGS: usize = 12;
    pub const PTRACE_SETREGS: usize = 13;
    pub const PTRACE_ATTACH: usize = 16;
    pub const PTRACE_DETACH: usize = 17;
    pub const PTRACE_SYSCALL: usize = 24;
    pub const PTRACE_SETOPTIONS: usize = 0x4200;
    pub const PTRACE_GETEVENTMSG: usize = 0x4201;
    pub const PTRACE_GETSIGINFO: usize = 0x4202;
    pub const PTRACE_SETSIGINFO: usize = 0x4203;
    pub const PTRACE_INTERRUPT: usize = 0x4207;
    pub const PTRACE_O_TRACESYSGOOD: usize = 1;
    pub const PTRACE_O_TRACEFORK: usize = 1 << 1;
    pub const PTRACE_O_TRACECLONE: usize = 1 << 3;
    pub const PTRACE_O_TRACEEXEC: usize = 1 << 4;
    pub const PTRACE_O_TRACEEXIT: usize = 1 << 6;
}

/// macOS 平台的 `ptrace` 请求号常量。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-unix"` 特性。
#[cfg(target_os = "macos")]
pub mod ptrace_req {
    pub const PT_TRACE_ME: usize = 0;
    pub const PT_READ_I: usize = 1;
    pub const PT_READ_D: usize = 2;
    pub const PT_WRITE_I: usize = 4;
    pub const PT_WRITE_D: usize = 5;
    pub const PT_CONTINUE: usize = 7;
    pub const PT_KILL: usize = 8;
    pub const PT_STEP: usize = 9;
    pub const PT_ATTACH: usize = 10;
    pub const PT_DETACH: usize = 11;
    pub const PT_SIGEXC: usize = 12;
    pub const PT_THUPDATE: usize = 13;
    pub const PT_ATTACHEXC: usize = 14;
    pub const PT_DENY_ATTACH: usize = 31;
}

/// 裸系统调用封装的统一返回类型。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-unix"` 特性。
///
/// # Errors
///
/// - 内核返回 `-4095..0` 范围内的负值时返回 [`SysErr::Ret`]，其余情况见各调用方的 `# Errors` 说明。
pub type SysResult<T = usize> = Result<T, SysErr>;

/// 裸系统调用封装的错误类型。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-unix"` 特性。
#[derive(Debug, Clone)]
pub enum SysErr {
    /// 操作系统返回的错误码 (errno)
    Ret(isize),
    /// 参数错误
    // Arg(String),
    Arg(ErrorMessage),
}
/// 定容 64 字节的错误消息，以 UTF-8 存储，超长截断。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-unix"` 特性。
///
/// # Examples
///
/// ```rust
/// use core::str::FromStr;
/// use lib_unknown::sys::unix::syscall::ErrorMessage;
///
/// let msg: ErrorMessage = "bad fd".parse().unwrap();
/// assert_eq!(msg.as_str(), "bad fd");
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ErrorMessage {
    buf: [u8; 64],
    len: u8,
}
impl ErrorMessage {
    /// 以 `&str` 形式查看消息内容。
    ///
    /// # Feature Requirement
    ///
    /// 需要启用 `"sys-unix"` 特性。
    ///
    /// # Examples
    ///
    /// ```rust
    /// use core::str::FromStr;
    /// use lib_unknown::sys::unix::syscall::ErrorMessage;
    ///
    /// let msg: ErrorMessage = "bad fd".parse().unwrap();
    /// assert_eq!(msg.as_str(), "bad fd");
    /// ```
    pub fn as_str(&self) -> &str {
        // SAFETY: `buf[..len]` 仅由 `FromStr` 经合法 UTF-8 切片写入，`len` 不超过 64，
        // 因此该切片恒为合法 UTF-8。
        unsafe { core::str::from_utf8_unchecked(&self.buf[..self.len as usize]) }
    }
}

impl core::str::FromStr for ErrorMessage {
    type Err = core::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut result = Self {
            buf: [0; 64],
            len: 0,
        };
        let bytes = s.as_bytes();
        let len = bytes.len().min(64);
        result.buf[..len].copy_from_slice(&bytes[..len]);
        result.len = len as u8;
        Ok(result)
    }
}

impl Display for SysErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            SysErr::Ret(errno) => write!(f, "OS System Call Error (errno: {})", errno),
            SysErr::Arg(msg) => write!(f, "Invalid Argument: {}", msg.as_str()),
        }
    }
}

impl Error for SysErr {}

#[inline(always)]
fn as_sys_result(ret: isize) -> SysResult {
    if (-4095..0).contains(&ret) {
        // Linux 标准 errno 范围
        Err(SysErr::Ret(-ret))
    } else {
        Ok(ret as usize)
    }
}

/// 退出当前进程，不返回。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-unix"` 特性。
///
/// # Examples
///
/// ```rust,no_run
/// use lib_unknown::sys::unix::syscall::sys_exit;
///
/// sys_exit(0);
/// ```
#[inline(always)]
pub fn sys_exit(status: usize) -> ! {
    // SAFETY: `EXIT` 为合法调用号，`status` 为按值传递的退出码，无指针参数。
    unsafe {
        syscall1(constants::EXIT, status);
    }
    ::core::unreachable!("exit syscall should not return")
}

/// 获取当前进程 ID。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-unix"` 特性。
///
/// # Examples
///
/// ```rust
/// use lib_unknown::sys::unix::syscall::sys_getpid;
///
/// let pid = sys_getpid();
/// assert!(pid > 0);
/// ```
#[inline(always)]
pub fn sys_getpid() -> usize {
    // SAFETY: `GETPID` 为合法调用号，无参数，不触碰用户内存。
    unsafe { syscall0(constants::GETPID) as usize }
}

/// 获取当前线程 ID（macOS 下为 `THREAD_SELFID`）。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-unix"` 特性。
///
/// # Examples
///
/// ```rust
/// use lib_unknown::sys::unix::syscall::sys_gettid;
///
/// let tid = sys_gettid();
/// assert!(tid > 0);
/// ```
#[inline(always)]
pub fn sys_gettid() -> usize {
    #[cfg(target_os = "macos")]
    // SAFETY: `THREAD_SELFID` 为合法调用号，无参数，不触碰用户内存。
    unsafe {
        syscall0(constants::THREAD_SELFID) as usize
    }
    #[cfg(not(target_os = "macos"))]
    // SAFETY: `GETTID` 为合法调用号，无参数，不触碰用户内存。
    unsafe {
        syscall0(constants::GETTID) as usize
    }
}

/// 从文件描述符读取至多 `buf.len()` 字节。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-unix"` 特性。
///
/// # Examples
///
/// ```rust,no_run
/// use lib_unknown::sys::unix::syscall::sys_read;
///
/// let mut buf = [0u8; 16];
/// let _ = sys_read(0, &mut buf);
/// ```
///
/// # Errors
///
/// - 当 `fd` 无效或不可读时返回 [`SysErr::Ret`]（内核 errno）。
#[inline(always)]
pub fn sys_read(fd: usize, buf: &mut [u8]) -> SysResult {
    // SAFETY: `READ` 为合法调用号；`buf` 在调用期间有效且可写，长度如实传递。
    unsafe {
        as_sys_result(syscall3(
            constants::READ,
            fd,
            buf.as_mut_ptr() as usize,
            buf.len(),
        ))
    }
}

/// 向文件描述符写入 `buf` 的全部内容（单次调用，不保证写完）。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-unix"` 特性。
///
/// # Examples
///
/// ```rust,no_run
/// use lib_unknown::sys::unix::syscall::sys_write;
///
/// let _ = sys_write(1, b"hi");
/// ```
///
/// # Errors
///
/// - 当 `fd` 无效或不可写时返回 [`SysErr::Ret`]（内核 errno）。
#[inline(always)]
pub fn sys_write(fd: usize, buf: &[u8]) -> SysResult {
    // SAFETY: `WRITE` 为合法调用号；`buf` 在调用期间有效且可读，长度如实传递。
    unsafe {
        as_sys_result(syscall3(
            constants::WRITE,
            fd,
            buf.as_ptr() as usize,
            buf.len(),
        ))
    }
}

/// 关闭文件描述符。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-unix"` 特性。
///
/// # Examples
///
/// ```rust,no_run
/// use lib_unknown::sys::unix::syscall::sys_close;
///
/// let _ = sys_close(3);
/// ```
///
/// # Errors
///
/// - 当 `fd` 不是已打开的描述符时返回 [`SysErr::Ret`]（内核 errno）。
#[inline(always)]
pub fn sys_close(fd: usize) -> SysResult {
    // SAFETY: `CLOSE` 为合法调用号，`fd` 按值传递，无指针参数。
    unsafe { as_sys_result(syscall1(constants::CLOSE, fd)) }
}

/// 以 `flags`/`mode` 打开 `path` 指向的路径，返回文件描述符。
///
/// aarch64/riscv64 上经 `OPENAT + AT_FDCWD` 实现，其余经 `OPEN` 实现。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-unix"` 特性。
///
/// # Examples
///
/// ```rust,no_run
/// use core::ffi::CStr;
/// use lib_unknown::sys::unix::syscall::{flags, sys_open};
///
/// let path = CStr::from_bytes_with_nul(b"/tmp\0").unwrap();
/// let _ = sys_open(path, flags::O_RDONLY, 0);
/// ```
///
/// # Errors
///
/// - 当路径不存在、无权限或 `flags` 非法时返回 [`SysErr::Ret`]（内核 errno）。
#[inline(always)]
pub fn sys_open(path: &CStr, flags: usize, mode: usize) -> SysResult {
    #[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
    // SAFETY: 调用号与参数顺序符合目标 ABI；`path` 为合法 NUL 结尾 C 字符串，
    // 在调用期间有效；`AT_FDCWD` 按值传递。
    unsafe {
        as_sys_result(syscall4(
            constants::OPENAT,
            flags::AT_FDCWD,
            path.as_ptr() as usize,
            flags,
            mode,
        ))
    }
    #[cfg(not(any(target_arch = "aarch64", target_arch = "riscv64")))]
    // SAFETY: 调用号与参数顺序符合目标 ABI；`path` 为合法 NUL 结尾 C 字符串，
    // 在调用期间有效。
    unsafe {
        as_sys_result(syscall3(
            constants::OPEN,
            path.as_ptr() as usize,
            flags,
            mode,
        ))
    }
}

/// 重定位文件描述符的读写偏移。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-unix"` 特性。
///
/// # Examples
///
/// ```rust,no_run
/// use lib_unknown::sys::unix::syscall::sys_lseek;
///
/// let _ = sys_lseek(0, 0, 0);
/// ```
///
/// # Errors
///
/// - 当 `fd` 不可定位或 `whence` 非法时返回 [`SysErr::Ret`]（内核 errno）。
#[inline(always)]
pub fn sys_lseek(fd: usize, offset: isize, whence: usize) -> SysResult {
    // SAFETY: `LSEEK` 为合法调用号，所有参数按值传递，无指针参数。
    unsafe { as_sys_result(syscall3(constants::LSEEK, fd, offset as usize, whence)) }
}

/// 建立内存映射，返回映射起始地址。
///
/// 32 位 x86 上经 `MMAP2` 实现（`offset` 须按页对齐），其余经 `MMAP` 实现。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-unix"` 特性。
///
/// # Examples
///
/// ```rust,no_run
/// use lib_unknown::sys::unix::syscall::{flags, sys_mmap};
///
/// let _ = sys_mmap(
///     core::ptr::null_mut(),
///     4096,
///     flags::PROT_READ | flags::PROT_WRITE,
///     flags::MAP_PRIVATE | flags::MAP_ANONYMOUS,
///     usize::MAX,
///     0,
/// );
/// ```
///
/// # Errors
///
/// - 当参数非法（如 `len` 为 0、无权限）时返回 [`SysErr::Ret`]（内核 errno）。
/// - 32 位 x86 上 `offset` 未按 4096 对齐时返回 [`SysErr::Arg`]。
#[inline(always)]
pub fn sys_mmap(
    addr: *mut u8,
    len: usize,
    prot: usize,
    flags: usize,
    fd: usize,
    offset: usize,
) -> SysResult {
    #[cfg(all(any(target_os = "linux", target_os = "android"), target_arch = "x86"))]
    {
        if offset % 4096 != 0 {
            const MSG: &str = "offset must be a multiple of 4096 on 32-bit x86";
            let bytes = MSG.as_bytes();
            let len = bytes.len().min(64);
            let mut buf = [0u8; 64];
            buf[..len].copy_from_slice(&bytes[..len]);
            return Err(SysErr::Arg(ErrorMessage {
                buf,
                len: len as u8,
            }));
        }
        // SAFETY: `MMAP2` 为合法调用号；`addr` 可为空（由内核选择地址），其余按值传递。
        unsafe {
            as_sys_result(syscall6(
                constants::MMAP2,
                addr as usize,
                len,
                prot,
                flags,
                fd,
                offset / 4096,
            ))
        }
    }
    #[cfg(not(all(any(target_os = "linux", target_os = "android"), target_arch = "x86")))]
    // SAFETY: `MMAP` 为合法调用号；`addr` 可为空（由内核选择地址），其余按值传递。
    unsafe {
        as_sys_result(syscall6(
            constants::MMAP,
            addr as usize,
            len,
            prot,
            flags,
            fd,
            offset,
        ))
    }
}

/// 解除由 [`sys_mmap`] 建立的内存映射。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-unix"` 特性。
///
/// # Examples
///
/// ```rust,no_run
/// use lib_unknown::sys::unix::syscall::sys_munmap;
///
/// let _ = sys_munmap(core::ptr::null_mut(), 4096);
/// ```
///
/// # Errors
///
/// - 当地址区间未映射时返回 [`SysErr::Ret`]（内核 errno）。
#[inline(always)]
pub fn sys_munmap(addr: *mut u8, len: usize) -> SysResult {
    // SAFETY: `MUNMAP` 为合法调用号；`addr`/`len` 应对应既有映射，按值传递。
    unsafe { as_sys_result(syscall2(constants::MUNMAP, addr as usize, len)) }
}

/// 修改既有内存映射的保护属性。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-unix"` 特性。
///
/// # Examples
///
/// ```rust,no_run
/// use lib_unknown::sys::unix::syscall::{flags, sys_mprotect};
///
/// let _ = sys_mprotect(core::ptr::null_mut(), 4096, flags::PROT_READ);
/// ```
///
/// # Errors
///
/// - 当地址区间未映射或 `prot` 非法时返回 [`SysErr::Ret`]（内核 errno）。
#[inline(always)]
pub fn sys_mprotect(addr: *mut u8, len: usize, prot: usize) -> SysResult {
    // SAFETY: `MPROTECT` 为合法调用号；`addr`/`len` 应对应既有映射，按值传递。
    unsafe { as_sys_result(syscall3(constants::MPROTECT, addr as usize, len, prot)) }
}

/// 创建套接字，返回文件描述符。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-unix"` 特性。
///
/// # Examples
///
/// ```rust,no_run
/// use lib_unknown::sys::unix::syscall::{flags, sys_socket};
///
/// let _ = sys_socket(flags::AF_INET, flags::SOCK_STREAM, 0);
/// ```
///
/// # Errors
///
/// - 当协议族/类型不支持时返回 [`SysErr::Ret`]（内核 errno）。
#[inline(always)]
pub fn sys_socket(domain: usize, ty: usize, protocol: usize) -> SysResult {
    // SAFETY: `SOCKET` 为合法调用号，所有参数按值传递，无指针参数。
    unsafe { as_sys_result(syscall3(constants::SOCKET, domain, ty, protocol)) }
}

/// 将套接字绑定到 `addr` 描述的地址上。
///
/// `addr` 应为 `sockaddr` 系列结构的原始字节。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-unix"` 特性。
///
/// # Examples
///
/// ```rust,no_run
/// use lib_unknown::sys::unix::syscall::sys_bind;
///
/// let addr = [0u8; 16];
/// let _ = sys_bind(3, &addr);
/// ```
///
/// # Errors
///
/// - 当 `fd` 非套接字或地址不可用时返回 [`SysErr::Ret`]（内核 errno）。
#[inline(always)]
pub fn sys_bind(fd: usize, addr: &[u8]) -> SysResult {
    // SAFETY: `BIND` 为合法调用号；`addr` 在调用期间有效且可读，长度如实传递。
    unsafe {
        as_sys_result(syscall3(
            constants::BIND,
            fd,
            addr.as_ptr() as usize,
            addr.len(),
        ))
    }
}

/// 使套接字进入监听状态。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-unix"` 特性。
///
/// # Examples
///
/// ```rust,no_run
/// use lib_unknown::sys::unix::syscall::sys_listen;
///
/// let _ = sys_listen(3, 16);
/// ```
///
/// # Errors
///
/// - 当 `fd` 非套接字或未绑定时返回 [`SysErr::Ret`]（内核 errno）。
#[inline(always)]
pub fn sys_listen(fd: usize, backlog: usize) -> SysResult {
    // SAFETY: `LISTEN` 为合法调用号，所有参数按值传递，无指针参数。
    unsafe { as_sys_result(syscall2(constants::LISTEN, fd, backlog)) }
}

/// 接受监听套接字上的连接，返回 `(新连接 fd, 对端地址长度)`。
///
/// 对端地址写入 `addr_buf`，实际长度经内核回写。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-unix"` 特性。
///
/// # Examples
///
/// ```rust,no_run
/// use lib_unknown::sys::unix::syscall::sys_accept;
///
/// let mut buf = [0u8; 32];
/// let _ = sys_accept(3, &mut buf);
/// ```
///
/// # Errors
///
/// - 当 `fd` 未监听或无待处理连接时返回 [`SysErr::Ret`]（内核 errno）。
#[inline(always)]
pub fn sys_accept(fd: usize, addr_buf: &mut [u8]) -> Result<(usize, usize), SysErr> {
    let mut addrlen: u32 = addr_buf.len() as u32;
    // SAFETY: `ACCEPT` 为合法调用号；`addr_buf` 在调用期间有效且可写，
    // `addrlen` 指向调用栈上的 `u32`，调用期间有效且可读写。
    let ret = unsafe {
        syscall3(
            constants::ACCEPT,
            fd,
            addr_buf.as_mut_ptr() as usize,
            &mut addrlen as *mut u32 as usize,
        )
    };
    let new_fd = as_sys_result(ret)?;
    Ok((new_fd, addrlen as usize))
}

/// 将套接字连接到 `addr` 描述的地址上。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-unix"` 特性。
///
/// # Examples
///
/// ```rust,no_run
/// use lib_unknown::sys::unix::syscall::sys_connect;
///
/// let addr = [0u8; 16];
/// let _ = sys_connect(3, &addr);
/// ```
///
/// # Errors
///
/// - 当目标不可达或连接被拒绝时返回 [`SysErr::Ret`]（内核 errno）。
#[inline(always)]
pub fn sys_connect(fd: usize, addr: &[u8]) -> SysResult {
    // SAFETY: `CONNECT` 为合法调用号；`addr` 在调用期间有效且可读，长度如实传递。
    unsafe {
        as_sys_result(syscall3(
            constants::CONNECT,
            fd,
            addr.as_ptr() as usize,
            addr.len(),
        ))
    }
}

/// 派生子进程，父进程返回子进程 PID，子进程返回 0。
///
/// aarch64/riscv64 上经 `CLONE + SIGCHLD`（栈传 0，Copy-On-Write）实现，
/// 其余经 `FORK` 实现。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-unix"` 特性。
///
/// # Examples
///
/// ```rust,no_run
/// use lib_unknown::sys::unix::syscall::sys_fork;
///
/// let _ = sys_fork();
/// ```
///
/// # Errors
///
/// - 当进程数达到上限或内存不足时返回 [`SysErr::Ret`]（内核 errno）。
#[inline(always)]
pub fn sys_fork() -> SysResult {
    #[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
    // SAFETY: `CLONE` 为合法调用号；仅传标志与空指针（clone 传 0 栈即 fork 语义），
    // 不触碰用户内存。
    // clone(flags, child_stack, parent_tidptr, tls, child_tidptr)
    // stack 传 0 会触发 Copy-On-Write，与传统的 fork 行为一致。
    unsafe {
        as_sys_result(syscall5(constants::CLONE, flags::SIGCHLD, 0, 0, 0, 0))
    }

    #[cfg(not(any(target_arch = "aarch64", target_arch = "riscv64")))]
    // SAFETY: `FORK` 为合法调用号，无参数，不触碰用户内存。
    unsafe {
        as_sys_result(syscall0(constants::FORK))
    }
}

/// 等待子进程状态变化，`status`/`rusage` 可为空指针表示不接收。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-unix"` 特性。
///
/// # Examples
///
/// ```rust,no_run
/// use lib_unknown::sys::unix::syscall::sys_wait4;
///
/// let _ = sys_wait4(-1, core::ptr::null_mut(), 0, core::ptr::null_mut());
/// ```
///
/// # Errors
///
/// - 当 `pid` 无效或无子进程时返回 [`SysErr::Ret`]（内核 errno）。
#[inline(always)]
pub fn sys_wait4(pid: isize, status: *mut i32, options: usize, rusage: *mut u8) -> SysResult {
    // SAFETY: `WAIT4` 为合法调用号；`status`/`rusage` 为空或指向调用方保证有效的
    // 可写内存，调用期间保持有效。
    unsafe {
        as_sys_result(syscall4(
            constants::WAIT4,
            pid as usize,
            status as usize,
            options,
            rusage as usize,
        ))
    }
}

/// 以 `argv`/`envp` 执行 `path` 指定的程序，成功时不返回。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-unix"` 特性。
///
/// # Examples
///
/// ```rust,no_run
/// use core::ffi::CStr;
/// use lib_unknown::sys::unix::syscall::sys_execve;
///
/// let path = CStr::from_bytes_with_nul(b"/bin/true\0").unwrap();
/// let argv: [*const u8; 1] = [path.as_ptr() as *const u8];
/// let envp: [*const u8; 0] = [];
/// let _ = sys_execve(path, &argv, &envp);
/// ```
///
/// # Errors
///
/// - 当程序不存在、无执行权限或参数非法时返回 [`SysErr::Ret`]（内核 errno）。
#[inline(always)]
pub fn sys_execve(path: &CStr, argv: &[*const u8], envp: &[*const u8]) -> SysResult {
    // SAFETY: `EXECVE` 为合法调用号；`path` 为合法 NUL 结尾字符串，`argv`/`envp`
    // 为调用期间有效的指针数组，其元素按目标约定指向合法字符串或为空。
    unsafe {
        as_sys_result(syscall3(
            constants::EXECVE,
            path.as_ptr() as usize,
            argv.as_ptr() as usize,
            envp.as_ptr() as usize,
        ))
    }
}

/// 发起 `ptrace` 调试请求，请求号见 [`ptrace_req`]。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-unix"` 特性。
///
/// # Examples
///
/// ```rust,no_run
/// use lib_unknown::sys::unix::syscall::{ptrace_req, sys_ptrace};
///
/// let _ = sys_ptrace(ptrace_req::PTRACE_TRACEME, 0, 0, 0);
/// ```
///
/// # Errors
///
/// - 当目标进程不存在、无权限或请求号非法时返回 [`SysErr::Ret`]（内核 errno）。
#[inline(always)]
pub fn sys_ptrace(request: usize, pid: isize, addr: usize, data: usize) -> SysResult {
    // SAFETY: `PTRACE` 为合法调用号；`request` 取自 `ptrace_req`，其余参数语义
    // 由具体请求号决定，按值传递。
    unsafe {
        as_sys_result(syscall4(
            constants::PTRACE,
            request,
            pid as usize,
            addr,
            data,
        ))
    }
}
