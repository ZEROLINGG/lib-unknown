// lib/src/sys/unix/syscall.rs

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

pub type SysResult<T = usize> = Result<T, SysErr>;

#[derive(Debug, Clone)]
pub enum SysErr {
    /// 操作系统返回的错误码 (errno)
    Ret(isize),
    /// 参数错误
    // Arg(String),
    Arg(ErrorMessage),
}
#[derive(Debug, Clone, Copy)]
pub struct ErrorMessage {
    buf: [u8; 64],
    len: u8,
}
impl ErrorMessage {
    pub fn as_str(&self) -> &str {
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

#[inline(always)]
pub fn sys_exit(status: usize) -> ! {
    unsafe {
        syscall1(constants::EXIT, status);
    }
    ::core::unreachable!("exit syscall should not return")
}

#[inline(always)]
pub fn sys_getpid() -> usize {
    unsafe { syscall0(constants::GETPID) as usize }
}

#[inline(always)]
pub fn sys_gettid() -> usize {
    #[cfg(target_os = "macos")]
    unsafe {
        syscall0(constants::THREAD_SELFID) as usize
    }
    #[cfg(not(target_os = "macos"))]
    unsafe {
        syscall0(constants::GETTID) as usize
    }
}

#[inline(always)]
pub fn sys_read(fd: usize, buf: &mut [u8]) -> SysResult {
    unsafe {
        as_sys_result(syscall3(
            constants::READ,
            fd,
            buf.as_mut_ptr() as usize,
            buf.len(),
        ))
    }
}

#[inline(always)]
pub fn sys_write(fd: usize, buf: &[u8]) -> SysResult {
    unsafe {
        as_sys_result(syscall3(
            constants::WRITE,
            fd,
            buf.as_ptr() as usize,
            buf.len(),
        ))
    }
}

#[inline(always)]
pub fn sys_close(fd: usize) -> SysResult {
    unsafe { as_sys_result(syscall1(constants::CLOSE, fd)) }
}

#[inline(always)]
pub fn sys_open(path: &CStr, flags: usize, mode: usize) -> SysResult {
    #[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
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
    unsafe {
        as_sys_result(syscall3(
            constants::OPEN,
            path.as_ptr() as usize,
            flags,
            mode,
        ))
    }
}

#[inline(always)]
pub fn sys_lseek(fd: usize, offset: isize, whence: usize) -> SysResult {
    unsafe { as_sys_result(syscall3(constants::LSEEK, fd, offset as usize, whence)) }
}

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
            return Err(SysErr::Arg(
                "offset must be a multiple of 4096 on 32-bit x86".to_string(),
            ));
        }
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

#[inline(always)]
pub fn sys_munmap(addr: *mut u8, len: usize) -> SysResult {
    unsafe { as_sys_result(syscall2(constants::MUNMAP, addr as usize, len)) }
}

#[inline(always)]
pub fn sys_mprotect(addr: *mut u8, len: usize, prot: usize) -> SysResult {
    unsafe { as_sys_result(syscall3(constants::MPROTECT, addr as usize, len, prot)) }
}

#[inline(always)]
pub fn sys_socket(domain: usize, ty: usize, protocol: usize) -> SysResult {
    unsafe { as_sys_result(syscall3(constants::SOCKET, domain, ty, protocol)) }
}

#[inline(always)]
pub fn sys_bind(fd: usize, addr: &[u8]) -> SysResult {
    unsafe {
        as_sys_result(syscall3(
            constants::BIND,
            fd,
            addr.as_ptr() as usize,
            addr.len(),
        ))
    }
}

#[inline(always)]
pub fn sys_listen(fd: usize, backlog: usize) -> SysResult {
    unsafe { as_sys_result(syscall2(constants::LISTEN, fd, backlog)) }
}

#[inline(always)]
pub fn sys_accept(fd: usize, addr_buf: &mut [u8]) -> Result<(usize, usize), SysErr> {
    let mut addrlen: u32 = addr_buf.len() as u32;
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

#[inline(always)]
pub fn sys_connect(fd: usize, addr: &[u8]) -> SysResult {
    unsafe {
        as_sys_result(syscall3(
            constants::CONNECT,
            fd,
            addr.as_ptr() as usize,
            addr.len(),
        ))
    }
}

#[inline(always)]
pub fn sys_fork() -> SysResult {
    #[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
    unsafe {
        // clone(flags, child_stack, parent_tidptr, tls, child_tidptr)
        // stack 传 0 会触发 Copy-On-Write，与传统的 fork 行为一致。
        as_sys_result(syscall5(constants::CLONE, flags::SIGCHLD, 0, 0, 0, 0))
    }

    #[cfg(not(any(target_arch = "aarch64", target_arch = "riscv64")))]
    unsafe {
        as_sys_result(syscall0(constants::FORK))
    }
}

#[inline(always)]
pub fn sys_wait4(pid: isize, status: *mut i32, options: usize, rusage: *mut u8) -> SysResult {
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

#[inline(always)]
pub fn sys_execve(path: &CStr, argv: &[*const u8], envp: &[*const u8]) -> SysResult {
    unsafe {
        as_sys_result(syscall3(
            constants::EXECVE,
            path.as_ptr() as usize,
            argv.as_ptr() as usize,
            envp.as_ptr() as usize,
        ))
    }
}

#[inline(always)]
pub fn sys_ptrace(request: usize, pid: isize, addr: usize, data: usize) -> SysResult {
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
