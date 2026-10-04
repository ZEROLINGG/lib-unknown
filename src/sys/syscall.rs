//! 裸系统调用 `syscall0~6`：Linux / Android / macOS / Windows 的多架构内联汇编分发，不支持的目标编译期报错。
//!
//! 需要启用 `"sys-syscall"` 特性。
#![no_std]
#![allow(unused)]

macro_rules! unsupported_target {
    ($name:literal) => {
        #[cfg(not(any(
            all(
                any(target_os = "linux", target_os = "android"),
                any(
                    target_arch = "x86_64",
                    target_arch = "aarch64",
                    target_arch = "riscv64",
                    target_arch = "x86"
                )
            ),
            all(
                target_os = "macos",
                any(target_arch = "x86_64", target_arch = "aarch64")
            ),
            all(
                target_os = "windows",
                any(target_arch = "x86_64", target_arch = "x86", target_arch = "aarch64")
            )
        )))]
        {
            compile_error!(concat!(
                $name,
                " is not supported on this target OS/Architecture combination."
            ));
            unreachable!()
        }
    };
}

/// 执行 0 参数裸系统调用。
///
/// # Safety
///
/// 调用此函数必须保证：
/// 1. `n` 为当前 OS/Arch 下合法的系统调用号；
/// 2. 调用者自行承担内核副作用（寄存器 `rcx/r11` 等按 ABI 被破坏）。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-syscall"` 特性。
///
/// # Examples
///
/// ```rust,no_run
/// use lib_unknown::sys::syscall::syscall0;
///
/// let pid = unsafe { syscall0(39) };
/// let _ = pid;
/// ```
#[inline(always)]
pub unsafe fn syscall0(n: usize) -> isize {
    let ret: isize;

    // =========================================================================
    // Linux & Android
    // =========================================================================

    #[cfg(all(
        any(target_os = "linux", target_os = "android"),
        target_arch = "x86_64"
    ))]
    unsafe {
        core::arch::asm!(
        "syscall",
        in("rax") n,
        lateout("rax") ret,
        lateout("rcx") _, // syscall 指令硬件语义：rcx 保存返回地址，被内核覆盖
        lateout("r11") _, // syscall 指令硬件语义：r11 保存 RFLAGS，被内核覆盖
        options(nostack)  // syscall 不触碰用户栈；内核可能修改 EFLAGS，故不加 preserves_flags
        );
    }

    #[cfg(all(
        any(target_os = "linux", target_os = "android"),
        target_arch = "aarch64"
    ))]
    unsafe {
        core::arch::asm!(
        "svc #0",
        in("x8") n,
        lateout("x0") ret,
        options(nostack) // Linux aarch64 syscall ABI 只保证修改 x0，其余寄存器保留
        );
    }

    #[cfg(all(
        any(target_os = "linux", target_os = "android"),
        target_arch = "riscv64"
    ))]
    unsafe {
        core::arch::asm!(
        "ecall",
        in("a7") n,
        lateout("a0") ret,
        options(nostack) // 同理，Linux riscv64 syscall 只保证修改 a0
        );
    }

    #[cfg(all(any(target_os = "linux", target_os = "android"), target_arch = "x86"))]
    unsafe {
        core::arch::asm!(
        "int 0x80",
        in("eax") n,
        lateout("eax") ret,
        options(nostack)
        );
    }

    // =========================================================================
    // macOS (Darwin)
    // =========================================================================

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    unsafe {
        let sys_num = n | 0x2000000;
        core::arch::asm!(
        "syscall",
        "jnc 1f",
        "neg rax",
        "1:",
        in("rax") sys_num,
        lateout("rax") ret,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack)
        );
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    unsafe {
        core::arch::asm!(
        "svc #0x80",
        "b.cc 1f",
        "neg x0, x0",
        "1:",
        in("x16") n,
        lateout("x0") ret,
        options(nostack)
        );
    }

    // =========================================================================
    // Windows (NT内核)
    // =========================================================================

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    unsafe {
        core::arch::asm!(
        "syscall",
        in("rax") n,
        lateout("rax") ret,
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r10") _,
        lateout("rdx") _,
        lateout("r8") _,
        lateout("r9") _,
        options(nostack)
        );
    }

    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    unsafe {
        core::arch::asm!(
        "int 0x2e",
        in("eax") n,
        lateout("eax") ret,
        lateout("ecx") _,
        lateout("edx") _,
        options(nostack)
        );
    }

    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    unsafe {
        core::arch::asm!(
        "svc #0",
        in("x8") n,
        lateout("x0") ret,
        options(nostack)
        );
    }

    unsupported_target!("syscall0");

    ret
}

/// 执行 1 参数裸系统调用。
///
/// # Safety
///
/// 调用此函数必须保证：
/// 1. `n` 合法且 `a1` 满足该调用的指针/值约束；
/// 2. 若 `a1` 为指针，调用期间其指向内存有效。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-syscall"` 特性.
#[inline(always)]
pub unsafe fn syscall1(n: usize, a1: usize) -> isize {
    let ret: isize;

    #[cfg(all(
        any(target_os = "linux", target_os = "android"),
        target_arch = "x86_64"
    ))]
    unsafe {
        core::arch::asm!(
        "syscall",
        in("rax") n,
        in("rdi") a1,
        lateout("rax") ret,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack)
        );
    }

    #[cfg(all(
        any(target_os = "linux", target_os = "android"),
        target_arch = "aarch64"
    ))]
    unsafe {
        core::arch::asm!(
        "svc #0",
        in("x8") n,
        inlateout("x0") a1 => ret,
        options(nostack)
        );
    }

    #[cfg(all(
        any(target_os = "linux", target_os = "android"),
        target_arch = "riscv64"
    ))]
    unsafe {
        core::arch::asm!(
        "ecall",
        in("a7") n,
        inlateout("a0") a1 => ret,
        options(nostack)
        );
    }

    #[cfg(all(any(target_os = "linux", target_os = "android"), target_arch = "x86"))]
    unsafe {
        core::arch::asm!(
        "int 0x80",
        in("eax") n,
        in("ebx") a1,
        lateout("eax") ret,
        options(nostack)
        );
    }

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    unsafe {
        let sys_num = n | 0x2000000;
        core::arch::asm!(
        "syscall",
        "jnc 1f",
        "neg rax",
        "1:",
        in("rax") sys_num,
        in("rdi") a1,
        lateout("rax") ret,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack)
        );
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    unsafe {
        core::arch::asm!(
        "svc #0x80",
        "b.cc 1f",
        "neg x0, x0",
        "1:",
        in("x16") n,
        inlateout("x0") a1 => ret,
        options(nostack)
        );
    }

    // Windows x64 NT syscall：第 1 个参数放在 r10（因 rcx 被 syscall 指令占用作
    // 返回地址）。内核将 r10/rdx/r8/r9 视为易失寄存器（scratch），并不保证
    // 保留其原值，因此对"作为输入使用过"的寄存器必须用 inout(..)=>_ 声明为
    // "输入后即作废"，未使用到的则用 lateout(_) 声明为纯粹的破坏。
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    unsafe {
        core::arch::asm!(
        "syscall",
        in("rax") n,
        inout("r10") a1 => _,
        lateout("rax") ret,
        lateout("rcx") _,
        lateout("r11") _,
        lateout("rdx") _,
        lateout("r8") _,
        lateout("r9") _,
        options(nostack)
        );
    }

    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    unsafe {
        core::arch::asm!(
        "push {a1}",
        "mov edx, esp",
        "int 0x2e",
        "add esp, 4",
        a1 = in(reg) a1,
        inout("eax") n => ret,
        lateout("ecx") _,
        lateout("edx") _,
        );
    }

    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    unsafe {
        core::arch::asm!(
        "svc #0",
        in("x8") n,
        inlateout("x0") a1 => ret,
        options(nostack)
        );
    }

    unsupported_target!("syscall1");

    ret
}

/// 执行 2 参数裸系统调用。
///
/// # Safety
///
/// 调用此函数必须保证 `n/a1/a2` 符合目标调用约定，指针参数有效。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-syscall"` 特性。
#[inline(always)]
pub unsafe fn syscall2(n: usize, a1: usize, a2: usize) -> isize {
    let ret: isize;

    #[cfg(all(
        any(target_os = "linux", target_os = "android"),
        target_arch = "x86_64"
    ))]
    unsafe {
        core::arch::asm!(
        "syscall",
        in("rax") n,
        in("rdi") a1,
        in("rsi") a2,
        lateout("rax") ret,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack)
        );
    }

    #[cfg(all(
        any(target_os = "linux", target_os = "android"),
        target_arch = "aarch64"
    ))]
    unsafe {
        core::arch::asm!(
        "svc #0",
        in("x8") n,
        inlateout("x0") a1 => ret,
        in("x1") a2,
        options(nostack)
        );
    }

    #[cfg(all(
        any(target_os = "linux", target_os = "android"),
        target_arch = "riscv64"
    ))]
    unsafe {
        core::arch::asm!(
        "ecall",
        in("a7") n,
        inlateout("a0") a1 => ret,
        in("a1") a2,
        options(nostack)
        );
    }

    #[cfg(all(any(target_os = "linux", target_os = "android"), target_arch = "x86"))]
    unsafe {
        core::arch::asm!(
        "int 0x80",
        in("eax") n,
        in("ebx") a1,
        in("ecx") a2,
        lateout("eax") ret,
        options(nostack)
        );
    }

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    unsafe {
        let sys_num = n | 0x2000000;
        core::arch::asm!(
        "syscall",
        "jnc 1f",
        "neg rax",
        "1:",
        in("rax") sys_num,
        in("rdi") a1,
        in("rsi") a2,
        lateout("rax") ret,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack)
        );
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    unsafe {
        core::arch::asm!(
        "svc #0x80",
        "b.cc 1f",
        "neg x0, x0",
        "1:",
        in("x16") n,
        inlateout("x0") a1 => ret,
        in("x1") a2,
        options(nostack)
        );
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    unsafe {
        core::arch::asm!(
        "syscall",
        in("rax") n,
        inout("r10") a1 => _,
        inout("rdx") a2 => _,
        lateout("rax") ret,
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r8") _,
        lateout("r9") _,
        options(nostack)
        );
    }

    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    unsafe {
        core::arch::asm!(
        "push {a2}",
        "push {a1}",
        "mov edx, esp",
        "int 0x2e",
        "add esp, 8",
        a1 = in(reg) a1,
        a2 = in(reg) a2,
        inout("eax") n => ret,
        lateout("ecx") _,
        lateout("edx") _,
        );
    }

    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    unsafe {
        core::arch::asm!(
        "svc #0",
        in("x8") n,
        inlateout("x0") a1 => ret,
        in("x1") a2,
        options(nostack)
        );
    }

    unsupported_target!("syscall2");

    ret
}

/// 执行 3 参数裸系统调用。
///
/// # Safety
///
/// 调用此函数必须保证参数符合目标调用约定，指针参数有效。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-syscall"` 特性。
#[inline(always)]
pub unsafe fn syscall3(n: usize, a1: usize, a2: usize, a3: usize) -> isize {
    let ret: isize;

    #[cfg(all(
        any(target_os = "linux", target_os = "android"),
        target_arch = "x86_64"
    ))]
    unsafe {
        core::arch::asm!(
        "syscall",
        in("rax") n,
        in("rdi") a1,
        in("rsi") a2,
        in("rdx") a3,
        lateout("rax") ret,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack)
        );
    }

    #[cfg(all(
        any(target_os = "linux", target_os = "android"),
        target_arch = "aarch64"
    ))]
    unsafe {
        core::arch::asm!(
        "svc #0",
        in("x8") n,
        inlateout("x0") a1 => ret,
        in("x1") a2,
        in("x2") a3,
        options(nostack)
        );
    }

    #[cfg(all(
        any(target_os = "linux", target_os = "android"),
        target_arch = "riscv64"
    ))]
    unsafe {
        core::arch::asm!(
        "ecall",
        in("a7") n,
        inlateout("a0") a1 => ret,
        in("a1") a2,
        in("a2") a3,
        options(nostack)
        );
    }

    #[cfg(all(any(target_os = "linux", target_os = "android"), target_arch = "x86"))]
    unsafe {
        core::arch::asm!(
        "int 0x80",
        in("eax") n,
        in("ebx") a1,
        in("ecx") a2,
        in("edx") a3,
        lateout("eax") ret,
        options(nostack)
        );
    }

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    unsafe {
        let sys_num = n | 0x2000000;
        core::arch::asm!(
        "syscall",
        "jnc 1f",
        "neg rax",
        "1:",
        in("rax") sys_num,
        in("rdi") a1,
        in("rsi") a2,
        in("rdx") a3,
        lateout("rax") ret,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack)
        );
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    unsafe {
        core::arch::asm!(
        "svc #0x80",
        "b.cc 1f",
        "neg x0, x0",
        "1:",
        in("x16") n,
        inlateout("x0") a1 => ret,
        in("x1") a2,
        in("x2") a3,
        options(nostack)
        );
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    unsafe {
        core::arch::asm!(
        "syscall",
        in("rax") n,
        inout("r10") a1 => _,
        inout("rdx") a2 => _,
        inout("r8") a3 => _,
        lateout("rax") ret,
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r9") _,
        options(nostack)
        );
    }

    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    unsafe {
        core::arch::asm!(
        "push {a3}",
        "push {a2}",
        "push {a1}",
        "mov edx, esp",
        "int 0x2e",
        "add esp, 12",
        a1 = in(reg) a1,
        a2 = in(reg) a2,
        a3 = in(reg) a3,
        inout("eax") n => ret,
        lateout("ecx") _,
        lateout("edx") _,
        );
    }

    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    unsafe {
        core::arch::asm!(
        "svc #0",
        in("x8") n,
        inlateout("x0") a1 => ret,
        in("x1") a2,
        in("x2") a3,
        options(nostack)
        );
    }

    unsupported_target!("syscall3");

    ret
}

/// 执行 4 参数裸系统调用。
///
/// # Safety
///
/// 调用此函数必须保证参数符合目标调用约定，指针参数有效。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-syscall"` 特性。
#[inline(always)]
pub unsafe fn syscall4(n: usize, a1: usize, a2: usize, a3: usize, a4: usize) -> isize {
    let ret: isize;

    #[cfg(all(
        any(target_os = "linux", target_os = "android"),
        target_arch = "x86_64"
    ))]
    unsafe {
        core::arch::asm!(
        "syscall",
        in("rax") n,
        in("rdi") a1,
        in("rsi") a2,
        in("rdx") a3,
        in("r10") a4,
        lateout("rax") ret,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack)
        );
    }

    #[cfg(all(
        any(target_os = "linux", target_os = "android"),
        target_arch = "aarch64"
    ))]
    unsafe {
        core::arch::asm!(
        "svc #0",
        in("x8") n,
        inlateout("x0") a1 => ret,
        in("x1") a2,
        in("x2") a3,
        in("x3") a4,
        options(nostack)
        );
    }

    #[cfg(all(
        any(target_os = "linux", target_os = "android"),
        target_arch = "riscv64"
    ))]
    unsafe {
        core::arch::asm!(
        "ecall",
        in("a7") n,
        inlateout("a0") a1 => ret,
        in("a1") a2,
        in("a2") a3,
        in("a3") a4,
        options(nostack)
        );
    }

    // esi 被 LLVM 内部占用，不可作为显式操作数，只能手动 push/mov/pop 传递，
    // 因此不能再用 options(nostack)。
    #[cfg(all(any(target_os = "linux", target_os = "android"), target_arch = "x86"))]
    unsafe {
        core::arch::asm!(
        "push esi",
        "mov esi, {a4}",
        "int 0x80",
        "pop esi",
        a4 = in(reg) a4,
        in("eax") n,
        in("ebx") a1,
        in("ecx") a2,
        in("edx") a3,
        lateout("eax") ret,
        );
    }

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    unsafe {
        let sys_num = n | 0x2000000;
        core::arch::asm!(
        "syscall",
        "jnc 1f",
        "neg rax",
        "1:",
        in("rax") sys_num,
        in("rdi") a1,
        in("rsi") a2,
        in("rdx") a3,
        in("r10") a4,
        lateout("rax") ret,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack)
        );
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    unsafe {
        core::arch::asm!(
        "svc #0x80",
        "b.cc 1f",
        "neg x0, x0",
        "1:",
        in("x16") n,
        inlateout("x0") a1 => ret,
        in("x1") a2,
        in("x2") a3,
        in("x3") a4,
        options(nostack)
        );
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    unsafe {
        core::arch::asm!(
        "syscall",
        in("rax") n,
        inout("r10") a1 => _,
        inout("rdx") a2 => _,
        inout("r8") a3 => _,
        inout("r9") a4 => _,
        lateout("rax") ret,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack)
        );
    }

    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    unsafe {
        core::arch::asm!(
        "push {a4}",
        "push {a3}",
        "push {a2}",
        "push {a1}",
        "mov edx, esp",
        "int 0x2e",
        "add esp, 16",
        a1 = in(reg) a1,
        a2 = in(reg) a2,
        a3 = in(reg) a3,
        a4 = in(reg) a4,
        inout("eax") n => ret,
        lateout("ecx") _,
        lateout("edx") _,
        );
    }

    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    unsafe {
        core::arch::asm!(
        "svc #0",
        in("x8") n,
        inlateout("x0") a1 => ret,
        in("x1") a2,
        in("x2") a3,
        in("x3") a4,
        options(nostack)
        );
    }

    unsupported_target!("syscall4");

    ret
}

/// 执行 5 参数裸系统调用。
///
/// # Safety
///
/// 调用此函数必须保证参数符合目标调用约定；Windows x64 下会临时扩展栈 `0x30` 字节。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-syscall"` 特性。
#[inline(always)]
pub unsafe fn syscall5(n: usize, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize) -> isize {
    let ret: isize;

    #[cfg(all(
        any(target_os = "linux", target_os = "android"),
        target_arch = "x86_64"
    ))]
    unsafe {
        core::arch::asm!(
        "syscall",
        in("rax") n,
        in("rdi") a1,
        in("rsi") a2,
        in("rdx") a3,
        in("r10") a4,
        in("r8")  a5,
        lateout("rax") ret,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack)
        );
    }

    #[cfg(all(
        any(target_os = "linux", target_os = "android"),
        target_arch = "aarch64"
    ))]
    unsafe {
        core::arch::asm!(
        "svc #0",
        in("x8") n,
        inlateout("x0") a1 => ret,
        in("x1") a2,
        in("x2") a3,
        in("x3") a4,
        in("x4") a5,
        options(nostack)
        );
    }

    #[cfg(all(
        any(target_os = "linux", target_os = "android"),
        target_arch = "riscv64"
    ))]
    unsafe {
        core::arch::asm!(
        "ecall",
        in("a7") n,
        inlateout("a0") a1 => ret,
        in("a1") a2,
        in("a2") a3,
        in("a3") a4,
        in("a4") a5,
        options(nostack)
        );
    }

    // esi 被 LLVM 内部占用，不可作为显式操作数，只能手动 push/mov/pop 传递，
    // 因此不能再用 options(nostack)。
    #[cfg(all(any(target_os = "linux", target_os = "android"), target_arch = "x86"))]
    unsafe {
        core::arch::asm!(
        "push esi",
        "mov esi, {a4}",
        "int 0x80",
        "pop esi",
        a4 = in(reg) a4,
        in("eax") n,
        in("ebx") a1,
        in("ecx") a2,
        in("edx") a3,
        in("edi") a5,
        lateout("eax") ret,
        );
    }

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    unsafe {
        let sys_num = n | 0x2000000;
        core::arch::asm!(
        "syscall",
        "jnc 1f",
        "neg rax",
        "1:",
        in("rax") sys_num,
        in("rdi") a1,
        in("rsi") a2,
        in("rdx") a3,
        in("r10") a4,
        in("r8")  a5,
        lateout("rax") ret,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack)
        );
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    unsafe {
        core::arch::asm!(
        "svc #0x80",
        "b.cc 1f",
        "neg x0, x0",
        "1:",
        in("x16") n,
        inlateout("x0") a1 => ret,
        in("x1") a2,
        in("x2") a3,
        in("x3") a4,
        in("x4") a5,
        options(nostack)
        );
    }

    // Windows x64 NT syscall：前 4 个参数走 r10/rdx/r8/r9（均需按输入后作废处理），
    // 第 5 个及以后的参数按 x64 调用约定放在 Shadow Space（0x20）之后的栈槽位，
    // 此处手动开辟 0x30 字节栈空间并写入，不能再用 options(nostack)。
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    unsafe {
        core::arch::asm!(
        "sub rsp, 0x30",
        "mov [rsp+0x20], {a5}",
        "syscall",
        "add rsp, 0x30",
        a5 = in(reg) a5,
        in("rax") n,
        inout("r10") a1 => _,
        inout("rdx") a2 => _,
        inout("r8") a3 => _,
        inout("r9") a4 => _,
        lateout("rax") ret,
        lateout("rcx") _,
        lateout("r11") _,
        );
    }

    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    unsafe {
        core::arch::asm!(
        "push {a5}",
        "push {a4}",
        "push {a3}",
        "push {a2}",
        "push {a1}",
        "mov edx, esp",
        "int 0x2e",
        "add esp, 20",
        a1 = in(reg) a1,
        a2 = in(reg) a2,
        a3 = in(reg) a3,
        a4 = in(reg) a4,
        a5 = in(reg) a5,
        inout("eax") n => ret,
        lateout("ecx") _,
        lateout("edx") _,
        );
    }

    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    unsafe {
        core::arch::asm!(
        "svc #0",
        in("x8") n,
        inlateout("x0") a1 => ret,
        in("x1") a2,
        in("x2") a3,
        in("x3") a4,
        in("x4") a5,
        options(nostack)
        );
    }

    unsupported_target!("syscall5");

    ret
}

/// 执行 6 参数裸系统调用。
///
/// # Safety
///
/// 调用此函数必须保证参数符合目标调用约定；x86 下会手动保存 `ebp`，Windows x64 下扩展栈。
///
/// # Feature Requirement
///
/// 需要启用 `"sys-syscall"` 特性。
#[inline(always)]
pub unsafe fn syscall6(
    n: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
    a6: usize,
) -> isize {
    let ret: isize;

    #[cfg(all(
        any(target_os = "linux", target_os = "android"),
        target_arch = "x86_64"
    ))]
    unsafe {
        core::arch::asm!(
        "syscall",
        in("rax") n,
        in("rdi") a1,
        in("rsi") a2,
        in("rdx") a3,
        in("r10") a4,
        in("r8")  a5,
        in("r9")  a6,
        lateout("rax") ret,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack)
        );
    }

    #[cfg(all(
        any(target_os = "linux", target_os = "android"),
        target_arch = "aarch64"
    ))]
    unsafe {
        core::arch::asm!(
        "svc #0",
        in("x8") n,
        inlateout("x0") a1 => ret,
        in("x1") a2,
        in("x2") a3,
        in("x3") a4,
        in("x4") a5,
        in("x5") a6,
        options(nostack)
        );
    }

    #[cfg(all(
        any(target_os = "linux", target_os = "android"),
        target_arch = "riscv64"
    ))]
    unsafe {
        core::arch::asm!(
        "ecall",
        in("a7") n,
        inlateout("a0") a1 => ret,
        in("a1") a2,
        in("a2") a3,
        in("a3") a4,
        in("a4") a5,
        in("a5") a6,
        options(nostack)
        );
    }

    // int 0x80 六参数约定的第 6 个参数放在 ebp，而 ebp/rbp 等帧指针寄存器被
    // rustc 显式禁止用作内联汇编的显式寄存器操作数（会直接编译报错），
    // 因此只能手动 push/pop 保存原 ebp，并通过通用寄存器模板传值。
    #[cfg(all(any(target_os = "linux", target_os = "android"), target_arch = "x86"))]
    unsafe {
        core::arch::asm!(
        "push ebp",
        "mov ebp, {a6}",
        "push esi",
        "mov esi, {a4}",
        "int 0x80",
        "pop esi",
        "pop ebp",
        a6 = in(reg) a6,
        a4 = in(reg) a4,
        inout("eax") n => ret,
        in("ebx") a1,
        in("ecx") a2,
        in("edx") a3,
        in("edi") a5,
        );
    }

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    unsafe {
        let sys_num = n | 0x2000000;
        core::arch::asm!(
        "syscall",
        "jnc 1f",
        "neg rax",
        "1:",
        in("rax") sys_num,
        in("rdi") a1,
        in("rsi") a2,
        in("rdx") a3,
        in("r10") a4,
        in("r8")  a5,
        in("r9")  a6,
        lateout("rax") ret,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack)
        );
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    unsafe {
        core::arch::asm!(
        "svc #0x80",
        "b.cc 1f",
        "neg x0, x0",
        "1:",
        in("x16") n,
        inlateout("x0") a1 => ret,
        in("x1") a2,
        in("x2") a3,
        in("x3") a4,
        in("x4") a5,
        in("x5") a6,
        options(nostack)
        );
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    unsafe {
        core::arch::asm!(
        "sub rsp, 0x30",
        "mov [rsp+0x20], {a5}",
        "mov [rsp+0x28], {a6}",
        "syscall",
        "add rsp, 0x30",
        a5 = in(reg) a5,
        a6 = in(reg) a6,
        in("rax") n,
        inout("r10") a1 => _,
        inout("rdx") a2 => _,
        inout("r8") a3 => _,
        inout("r9") a4 => _,
        lateout("rax") ret,
        lateout("rcx") _,
        lateout("r11") _,
        );
    }

    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    unsafe {
        core::arch::asm!(
        "push {a6}",
        "push {a5}",
        "push {a4}",
        "push {a3}",
        "push {a2}",
        "push {a1}",
        "mov edx, esp",
        "int 0x2e",
        "add esp, 24",
        a1 = in(reg) a1,
        a2 = in(reg) a2,
        a3 = in(reg) a3,
        a4 = in(reg) a4,
        a5 = in(reg) a5,
        a6 = in(reg) a6,
        inout("eax") n => ret,
        lateout("ecx") _,
        lateout("edx") _,
        );
    }

    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    unsafe {
        core::arch::asm!(
        "svc #0",
        in("x8") n,
        inlateout("x0") a1 => ret,
        in("x1") a2,
        in("x2") a3,
        in("x3") a4,
        in("x4") a5,
        in("x5") a6,
        options(nostack)
        );
    }

    unsupported_target!("syscall6");

    ret
}
