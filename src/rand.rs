//! 硬件熵随机数：时间戳熵源（`probe`）、重播种（`seed`）与热路径输出（`next`），另含区间采样与 `fill_bytes`（`rand-expand`）。
//!
//! 需要启用 `"rand"` 特性。
#![allow(unused)]

use crate::crypto::base::{mix64, shuffle_with};
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
// /dev/urandom参考
// ❯ head -c 10M /dev/urandom > small_samples.bin
// ❯ /home/zz/Documents/git/SP800-90B_EntropyAssessment/cpp/ea_non_iid -i -v -a small_samples.bin 8
// Opening file: 'small_samples.bin' (SHA-256 hash 4466b8477a570ab30889532da827030cc389125bd484d43f8339cf00a23c90bd)
// Loaded 10485760 samples of 256 distinct 8-bit-wide symbols
// Number of Binary Symbols: 83886080
//
// Running non-IID tests...
//
// Running Most Common Value Estimate...
// Bitstring MCV Estimate: mode = 41944627, p-hat = 0.50001891851425173, p_u = 0.50015953690906079
//         Most Common Value Estimate (bit string) = 0.999540 / 1 bit(s)
// Literal MCV Estimate: mode = 41721, p-hat = 0.003978824615478516, p_u = 0.0040289005228594642
//         Most Common Value Estimate = 7.955398 / 8 bit(s)
//
// Running Entropic Statistic Estimates (bit strings only)...
// Bitstring Collision Estimate: X-bar = 2.500075193522735, sigma-hat = 0.50000000179673953, p = 0.50857750043554861
//         Collision Test Estimate (bit string) = 0.975460 / 1 bit(s)
// Bitstring Markov Estimate: P_0 = 0.49998108148574827, P_1 = 0.50001891851425173, P_0,0 = 0.49996292450723928, P_0,1 = 0.50003707549276077, P_1,0 = 0.49999924900989107, P_1,1 = 0.50000075099010899, p_max = 2.952323724507251e-39
//         Markov Test Estimate (bit string) = 0.999948 / 1 bit(s)
// Bitstring Compression Estimate: X-bar = 5.2173562024019668, sigma-hat = 1.0155761472136569, p = 0.020720381304131674
//         Compression Test Estimate (bit string) = 0.932134 / 1 bit(s)
//
// Running Tuple Estimates...
// Bitstring t-Tuple Estimate: t = 22, p-hat_max = 0.519800544732285640763, p_u = 0.5199410528217940434775
// Bitstring LRS Estimate: u = 23, v = 49, p-hat = 0.50048298953775729, p_u = 0.50062360786706038
//         T-Tuple Test Estimate (bit string) = 0.943580 / 1 bit(s)
// Literal t-Tuple Estimate: t = 2, p-hat_max = 0.004590882747638149776254, p_u = 0.004644655968621724387299
// Literal LRS Estimate: u = 3, v = 5, p-hat = 0.0039149192648118801, p_u = 0.0039645929941324926
//         T-Tuple Test Estimate = 7.750213 / 8 bit(s)
//         LRS Test Estimate (bit string) = 0.998202 / 1 bit(s)
//         LRS Test Estimate = 7.978612 / 8 bit(s)
//
// Running Predictor Estimates...
// Bitstring MultiMCW Prediction Estimate: N = 83886017, Pglobal' = 0.50016892471455032 (C = 41945383) Plocal can't affect result (r = 26)
//         Multi Most Common in Window (MultiMCW) Prediction Test Estimate (bit string) = 0.999513 / 1 bit(s)
// Literal MultiMCW Prediction Estimate: N = 10485697, Pglobal' = 0.0039368902123073065 (C = 40762) Plocal = 0.0055718895656600631 (r = 4)
//         Multi Most Common in Window (MultiMCW) Prediction Test Estimate = 7.487618 / 8 bit(s)
// Bitstring Lag Prediction Estimate: N = 83886079, Pglobal' = 0.50024399072940884 (C = 41951711) Plocal can't affect result (r = 28)
//         Lag Prediction Test Estimate (bit string) = 0.999296 / 1 bit(s)
// Literal Lag Prediction Estimate: N = 10485759, Pglobal' = 0.003972280184949272 (C = 41131) Plocal = 0.0055718813178256762 (r = 4)
//         Lag Prediction Test Estimate = 7.487620 / 8 bit(s)
// Bitstring MultiMMC Prediction Estimate: N = 83886078, Pglobal' = 0.50009155185115994 (C = 41938923) Plocal can't affect result (r = 24)
//         Multi Markov Model with Counting (MultiMMC) Prediction Test Estimate (bit string) = 0.999736 / 1 bit(s)
// Literal MultiMMC Prediction Estimate: N = 10485758, Pglobal' = 0.0039454088113584813 (C = 40851) Plocal = 0.0055718814508428282 (r = 4)
//         Multi Markov Model with Counting (MultiMMC) Prediction Test Estimate = 7.487620 / 8 bit(s)
// Bitstring LZ78Y Prediction Estimate: N = 83886063, Pglobal' = 0.5000796607255481 (C = 41937918) Plocal can't affect result (r = 25)
//         LZ78Y Prediction Test Estimate (bit string) = 0.999770 / 1 bit(s)
// Literal LZ78Y Prediction Estimate: N = 10485743, Pglobal' = 0.0039455104269033141 (C = 40852) Plocal = 0.0055718834462567543 (r = 4)
//         LZ78Y Prediction Test Estimate = 7.487619 / 8 bit(s)
//
// H_original: 7.487618
// H_bitstring: 0.932134
// min(H_original, 8 X H_bitstring): 7.457074

// 主要熵源
/// 采集硬件时间戳并归一化到 1GHz 的原始熵源。
///
/// 读取 x86(`RDTSC`)/aarch64(`CNTVCT_EL0`)/RISC-V(`rdtime`) 等计数器，
/// 用 Q48 定点乘子换算为纳秒级单调时钟。
///
/// # Feature Requirement
///
/// 需要启用 `"rand"` 特性。
///
/// # Examples
///
/// ```rust
/// use lib_unknown::rand::probe;
///
/// let t1 = probe();
/// let t2 = probe();
/// assert!(t2 >= t1 || t2.wrapping_sub(t1) < u64::MAX);
/// ```
#[inline(always)]
pub fn probe() -> u64 {
    const SHIFT: u8 = 48;
    const TARGET_FREQ: u128 = 1_000_000_000; // 目标 1GHz (1 tick = 1 ns)
    static MULTIPLIER_Q48: AtomicU64 = AtomicU64::new(0);
    #[inline(always)]
    fn raw_probe() -> u64 {
        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        {
            #[cfg(target_arch = "x86_64")]
            unsafe {
                core::arch::x86_64::_rdtsc()
            }
            #[cfg(target_arch = "x86")]
            unsafe {
                core::arch::x86::_rdtsc()
            }
        }

        #[cfg(target_arch = "aarch64")]
        {
            let tsc: u64;
            unsafe {
                core::arch::asm!("mrs {}, cntvct_el0", out(reg) tsc, options(nomem, nostack))
            };
            tsc
        }

        #[cfg(target_arch = "arm")]
        {
            let mut lo: u32;
            let mut hi: u32;
            unsafe {
                core::arch::asm!("mrrc p15, 1, {}, {}, c14", out(reg) lo, out(reg) hi, options(nomem, nostack))
            };
            ((hi as u64) << 32) | (lo as u64)
        }

        #[cfg(target_arch = "riscv64")]
        {
            let tsc: u64;
            unsafe { core::arch::asm!("rdtime {}", out(reg) tsc, options(nomem, nostack)) };
            tsc
        }

        #[cfg(target_arch = "riscv32")]
        {
            let mut hi: u32;
            let mut lo: u32;
            let mut hi2: u32;
            unsafe {
                core::arch::asm!(
                "2:", "rdtimeh {hi}", "rdtime {lo}", "rdtimeh {hi2}", "bne {hi}, {hi2}, 2b",
                hi = out(reg) hi, lo = out(reg) lo, hi2 = out(reg) hi2, options(nomem, nostack, preserves_flags)
                );
            }
            ((hi as u64) << 32) | (lo as u64)
        }

        #[cfg(target_arch = "loongarch64")]
        {
            let tsc: u64;
            unsafe { core::arch::asm!("rdtime.d {}, $r0", out(reg) tsc, options(nomem, nostack)) };
            tsc
        }

        #[cfg(target_arch = "powerpc")]
        {
            let tsc: u64;
            unsafe { core::arch::asm!("mftb {}", out(reg) tsc, options(nomem, nostack)) };
            tsc
        }

        #[cfg(target_arch = "wasm32")]
        {
            #[cfg(target_os = "wasi")]
            {
                let mut timespec = core::mem::MaybeUninit::uninit();
                unsafe {
                    let _ = wasi::clock_time_get(wasi::CLOCKID_MONOTONIC, 1, timespec.as_mut_ptr());
                    timespec.assume_init()
                }
            }
            #[cfg(not(target_os = "wasi"))]
            {
                // Web 环境下，即使 no_std 也需要通过 ffi 引入 JS 运行时 API
                #[wasm_bindgen::prelude::wasm_bindgen]
                extern "C" {
                    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = performance)]
                    fn now() -> f64;
                }
                (now() * 1_000_000.0) as u64
            }
        }
    }

    fn autodetect_hardware_freq() -> u64 {
        #[cfg(target_arch = "aarch64")]
        {
            let freq: u64;
            unsafe {
                core::arch::asm!("mrs {}, cntfrq_el0", out(reg) freq, options(nomem, nostack))
            };
            return freq;
        }

        #[cfg(target_arch = "loongarch64")]
        {
            let freq: u32;
            // 龙芯 CPUCFG 指令索引 4，直接返回 CPU 定时器频率
            unsafe {
                core::arch::asm!("cpucfg {}, {}", out(reg) freq, in(reg) 4, options(nomem, nostack))
            };
            return freq as u64;
        }

        #[cfg(target_arch = "wasm32")]
        {
            return 1_000_000_000; // Web/WASI 已被我们在 raw_probe 中转为 1GHz
        }

        #[cfg(not(any(
            target_arch = "aarch64",
            target_arch = "loongarch64",
            target_arch = "wasm32"
        )))]
        {
            2_500_000_000
        }
    }

    let raw = raw_probe();
    let mut mult = MULTIPLIER_Q48.load(Ordering::Relaxed);

    if mult == 0 {
        let hz = autodetect_hardware_freq();
        let final_mult = if hz == 0 {
            1u64 << SHIFT
        } else {
            ((TARGET_FREQ << SHIFT) / hz as u128) as u64
        };
        mult = match MULTIPLIER_Q48.compare_exchange(
            0,
            final_mult,
            Ordering::SeqCst,
            Ordering::Relaxed,
        ) {
            Ok(_) => final_mult,
            Err(existing) => existing,
        };
    }

    ((raw as u128 * mult as u128) >> SHIFT) as u64
}

// 较低的熵源
#[inline(never)]
fn reg_sig() -> u64 {
    let state: u64;
    let gpr1: u64;
    let gpr2: u64;
    let gpr3: u64;
    let code: u64;
    let stack: u64;

    // ==========================================
    // 64位架构支持
    // ==========================================
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!(
        "pushfq",
        "pop {0}",
        "lea {1}, [rip]",
        "mov {2}, rsp",
        out(reg) state, out(reg) code, out(reg) stack,
        out("rcx") gpr1, out("r10") gpr2, out("r11") gpr3,
        );
    }

    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!(
        "mrs {0}, nzcv",
        "adr {1}, .",
        "mov {2}, sp",
        out(reg) state, out(reg) code, out(reg) stack,
        out("x1") gpr1, out("x2") gpr2, out("x3") gpr3,
        options(nomem, nostack)
        );
    }

    #[cfg(target_arch = "riscv64")]
    unsafe {
        core::arch::asm!(
        "rdinstret {0}",
        "auipc {1}, 0",
        "mv {2}, sp",
        out(reg) state, out(reg) code, out(reg) stack,
        out("t0") gpr1, out("t1") gpr2, out("t2") gpr3,
        options(nomem, nostack)
        );
    }

    #[cfg(target_arch = "x86")]
    unsafe {
        let (s32, c32, sp32, g1, g2, g3): (u32, u32, u32, u32, u32, u32);
        core::arch::asm!(
        "pushfd",
        "pop {0}",
        "call 2f",
        "2:",
        "pop {1}",
        "mov {2}, esp",
        out(reg) s32, out(reg) c32, out(reg) sp32,
        out("ecx") g1, out("edx") g2, out("edi") g3,
        );
        state = s32 as u64;
        code = c32 as u64;
        stack = sp32 as u64;
        gpr1 = g1 as u64;
        gpr2 = g2 as u64;
        gpr3 = g3 as u64;
    }

    #[cfg(target_arch = "arm")]
    unsafe {
        let (s32, c32, sp32, g1, g2, g3): (u32, u32, u32, u32, u32, u32);
        core::arch::asm!(
        "mrs {0}, CPSR", // 读取当前程序状态寄存器
        "mov {1}, pc",   // 32位ARM允许直接读取PC寄存器
        "mov {2}, sp",
        out(reg) s32, out(reg) c32, out(reg) sp32,
        out("r1") g1, out("r2") g2, out("r3") g3,
        options(nomem, nostack)
        );
        state = s32 as u64;
        code = c32 as u64;
        stack = sp32 as u64;
        gpr1 = g1 as u64;
        gpr2 = g2 as u64;
        gpr3 = g3 as u64;
    }

    #[cfg(target_arch = "riscv32")]
    unsafe {
        let (s32, c32, sp32, g1, g2, g3): (u32, u32, u32, u32, u32, u32);
        core::arch::asm!(
        "rdinstret {0}", // RISC-V 32同样支持退休指令计数器
        "auipc {1}, 0",
        "mv {2}, sp",
        out(reg) s32, out(reg) c32, out(reg) sp32,
        out("t0") g1, out("t1") g2, out("t2") g3,
        options(nomem, nostack)
        );
        state = s32 as u64;
        code = c32 as u64;
        stack = sp32 as u64;
        gpr1 = g1 as u64;
        gpr2 = g2 as u64;
        gpr3 = g3 as u64;
    }

    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64",
        target_arch = "x86",
        target_arch = "arm",
        target_arch = "riscv32"
    )))]
    {
        state = 0;
        gpr1 = 0;
        gpr2 = 0;
        gpr3 = 0;
        code = 0;
        stack = 0;
    }

    let mut s = state.rotate_left(13);
    s ^= gpr1.rotate_right(17);
    s ^= gpr2.rotate_left(11);
    s ^= gpr3.rotate_right(3);
    s ^= code.rotate_left(17);
    s ^= stack.rotate_right(5);
    mix64(s)
}

// 10MB seed()产出测试
// Loaded 10485760 samples of 256 distinct 8-bit-wide symbols
// Number of Binary Symbols: 83886080
//
// Running non-IID tests...
//
// Running Most Common Value Estimate...
// Bitstring MCV Estimate: mode = 41952873, p-hat = 0.50011721849441526, p_u = 0.50025783688546077
//         Most Common Value Estimate (bit string) = 0.999256 / 1 bit(s)
// Literal MCV Estimate: mode = 41493, p-hat = 0.0039570808410644533, p_u = 0.004007020276832818
//         Most Common Value Estimate = 7.963254 / 8 bit(s)
//
// Running Entropic Statistic Estimates (bit strings only)...
// Bitstring Collision Estimate: X-bar = 2.4998640044984755, sigma-hat = 0.4999999889553986, p = 0.51338519057718568
//         Collision Test Estimate (bit string) = 0.961886 / 1 bit(s)
// Bitstring Markov Estimate: P_0 = 0.49988278150558474, P_1 = 0.50011721849441526, P_0,0 = 0.49999291730758672, P_0,1 = 0.50000708269241323, P_1,0 = 0.49977268541298708, P_1,1 = 0.50022731458701286, p_max = 3.1140954007368964e-39
//         Markov Test Estimate (bit string) = 0.999347 / 1 bit(s)
// Bitstring Compression Estimate: X-bar = 5.2179921089068255, sigma-hat = 1.0151616484856194, p = 0.018763160395471878
//         Compression Test Estimate (bit string) = 0.955992 / 1 bit(s)
//
// Running Tuple Estimates...
// Bitstring t-Tuple Estimate: t = 22, p-hat_max = 0.5192926595069563040102, p_u = 0.5194331731846545434296
// Bitstring LRS Estimate: u = 23, v = 50, p-hat = 0.50285994550643263, p_u = 0.50300056160100674
//         T-Tuple Test Estimate (bit string) = 0.944990 / 1 bit(s)
// Literal t-Tuple Estimate: t = 2, p-hat_max = 0.004528134246922033372418, p_u = 0.004581540398724434760821
// Literal LRS Estimate: u = 3, v = 5, p-hat = 0.0039059863557037059, p_u = 0.0039556036033593406
//         T-Tuple Test Estimate = 7.769952 / 8 bit(s)
//         LRS Test Estimate (bit string) = 0.991368 / 1 bit(s)
//         LRS Test Estimate = 7.981886 / 8 bit(s)
//
// Running Predictor Estimates...
// Bitstring MultiMCW Prediction Estimate: N = 83886017, Pglobal' = 0.50017991581909249 (C = 41946305) Plocal can't affect result (r = 27)
//         Multi Most Common in Window (MultiMCW) Prediction Test Estimate (bit string) = 0.999481 / 1 bit(s)
// Literal MultiMCW Prediction Estimate: N = 10485697, Pglobal' = 0.0039609791527534998 (C = 41013) Plocal can't affect result (r = 3)
//         Multi Most Common in Window (MultiMCW) Prediction Test Estimate = 7.979927 / 8 bit(s)
// Bitstring Lag Prediction Estimate: N = 83886079, Pglobal' = 0.50005931169698126 (C = 41936219) Plocal can't affect result (r = 29)
//         Lag Prediction Test Estimate (bit string) = 0.999829 / 1 bit(s)
// Literal Lag Prediction Estimate: N = 10485759, Pglobal' = 0.0039561572050945742 (C = 40963) Plocal = 0.0055718813178256762 (r = 4)
//         Lag Prediction Test Estimate = 7.487620 / 8 bit(s)
// Bitstring MultiMMC Prediction Estimate: N = 83886078, Pglobal' = 0.50020497949238396 (C = 41948438) Plocal can't affect result (r = 26)
//         Multi Markov Model with Counting (MultiMMC) Prediction Test Estimate (bit string) = 0.999409 / 1 bit(s)
// Literal MultiMMC Prediction Estimate: N = 10485758, Pglobal' = 0.0039504952950308991 (C = 40904) Plocal = 0.0055718814508428282 (r = 4)
//         Multi Markov Model with Counting (MultiMMC) Prediction Test Estimate = 7.487620 / 8 bit(s)
// Bitstring LZ78Y Prediction Estimate: N = 83886063, Pglobal' = 0.5002201250591588 (C = 41949701) Plocal can't affect result (r = 28)
//         LZ78Y Prediction Test Estimate (bit string) = 0.999365 / 1 bit(s)
// Literal LZ78Y Prediction Estimate: N = 10485743, Pglobal' = 0.0039507888600843859 (C = 40907) Plocal = 0.0055718834462567543 (r = 4)
//         LZ78Y Prediction Test Estimate = 7.487619 / 8 bit(s)
//
// H_original: 7.487619
// H_bitstring: 0.944990
// min(H_original, 8 X H_bitstring): 7.487619

// RNG = RNG_stdin, seed = unknown
// test set = core, folding = standard(unknown format)
//
// rng=RNG_stdin, seed=unknown
// length= 32 megabytes (2^25 bytes), time= 2.6 seconds
//   no anomalies in 163 test result(s)
//
// rng=RNG_stdin, seed=unknown
// length= 64 megabytes (2^26 bytes), time= 5.5 seconds
//   no anomalies in 174 test result(s)
//
// rng=RNG_stdin, seed=unknown
// length= 128 megabytes (2^27 bytes), time= 11.1 seconds
//   no anomalies in 187 test result(s)
//
// rng=RNG_stdin, seed=unknown
// length= 256 megabytes (2^28 bytes), time= 21.8 seconds
//   no anomalies in 201 test result(s)
//
// rng=RNG_stdin, seed=unknown
// length= 512 megabytes (2^29 bytes), time= 43.1 seconds
//   no anomalies in 216 test result(s)
/// 混合多熵源生成 64 位种子。
///
/// 将 `probe` 时间戳、`reg_sig` 寄存器指纹与栈内存抖动混合搅拌，
/// 适用于 `next` 的定期重播种。开销高于 `next`，勿用于热路径。
///
/// # Feature Requirement
///
/// 需要启用 `"rand"` 特性。
///
/// # Examples
///
/// ```rust
/// use lib_unknown::rand::seed;
///
/// let s1 = seed();
/// let s2 = seed();
/// assert_ne!(s1, s2);
/// ```
pub fn seed() -> u64 {
    use core::hint::black_box;

    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    const STACK_MASK: usize = 767;
    static CONFIG: (u8, u8, u8) = (32, 16, 8);

    let c = COUNTER.fetch_add(1, Ordering::Relaxed) as u8;
    let x = probe();
    let y = reg_sig();
    #[cfg(feature = "rand-safe-stack")]
    let stack: core::mem::MaybeUninit<[u8; STACK_MASK]> = core::mem::MaybeUninit::uninit();
    let sp = unsafe {
        #[cfg(feature = "rand-safe-stack")]
        {
            black_box(stack.as_ptr() as *const u8)
        }
        #[cfg(not(feature = "rand-safe-stack"))]
        {
            black_box(&c as *const u8) // 大多数情况下rust程序使用的栈大于767字节
        }
    };
    let rs = |idx| unsafe { black_box(core::ptr::read_volatile(sp.add(idx))).saturating_add(2) };
    let t2s = |t: u64, mut s: u64, i: u8| {
        if t.rotate_left(i as u32) as u8 & 1 == 0 {
            let idx = ((t as u16) & (STACK_MASK as u16)) as usize;
            s = s.rotate_left(rs(idx) as u32);
            let idx2 = (((s >> 16) as u16) & (STACK_MASK as u16)) as usize;
            s = s.wrapping_mul(rs(idx2) as u64);
        } else {
            let idx = (((t >> 16) as u16) & (STACK_MASK as u16)) as usize;
            s = s.rotate_right(rs(idx) as u32);
            let idx2 = ((s as u16) & (STACK_MASK as u16)) as usize;
            s = s.wrapping_mul(rs(idx2) as u64);
        }

        s = match s.rotate_left(i as u32) as u8 {
            0 => s ^ 0x9e3779b97f4a7c15,
            255 => s.wrapping_mul(probe().saturating_add(2)),
            127 => s.rotate_right(7),
            3 => s.wrapping_mul(STACK_MASK as u64),
            7 => s.wrapping_add(0x9e3779b97f4a7c15),
            32 => s.wrapping_mul(0x94d049bb133111eb),
            _ => s,
        };
        s
    };

    let mut s = (x ^ y).rotate_right(c as u32);

    for i in 0..black_box(CONFIG.0) {
        if (s >> 8) as u8 > 127 {
            continue;
        }
        let t1 = probe().wrapping_mul((i as u64 ^ s).wrapping_mul(0x9e3779b97f4a7c15));
        s ^= t1;
        s = t2s(t1, s, i);

        for j in 0..black_box(CONFIG.1) {
            if (s >> 16) as u8 <= 127 {
                continue;
            }
            let t2 = probe().wrapping_mul(((i ^ j) as u64 ^ s).wrapping_mul(0x9e3779b97f4a7c15));
            s ^= t2;
            s = t2s(t2, s, i ^ j);

            for k in 0..black_box(CONFIG.2) {
                if (s >> 32) as u8 > 127 {
                    continue;
                }
                let t3 =
                    probe().wrapping_mul(((i ^ j ^ k) as u64 ^ s).wrapping_mul(0x9e3779b97f4a7c15));
                s ^= t3;
                s = t2s(t3, s, i ^ j ^ k);
            }
        }
    }
    mix64(s)
}

/// 使用系统熵原地打乱可变序列。
///
/// # Feature Requirement
///
/// 需要启用 `"rand"` 特性。
///
/// # Examples
///
/// ```rust
/// use lib_unknown::rand::shuffle;
///
/// let v = shuffle([0, 1, 2, 3, 4, 5, 6, 7]);
/// assert_eq!(v.len(), 8);
/// ```
pub fn shuffle<T, U>(mut table: U) -> U
where
    U: AsMut<[T]>,
{
    shuffle_with(table, next())
}

// RNG = RNG_stdin, seed = unknown
// test set = core, folding = standard(unknown format)
//
// rng=RNG_stdin, seed=unknown
// length= 256 megabytes (2^28 bytes), time= 3.6 seconds
//   no anomalies in 201 test result(s)
//
// rng=RNG_stdin, seed=unknown
// length= 512 megabytes (2^29 bytes), time= 7.4 seconds
//   Test Name                         Raw       Processed     Evaluation
//   [Low1/32]BCFN(2+1,13-5U)          R=  -7.0  p =1-1.8e-4   unusual
//   ...and 215 test result(s) without anomalies
//
// rng=RNG_stdin, seed=unknown
// length= 1 gigabyte (2^30 bytes), time= 14.7 seconds
//   no anomalies in 231 test result(s)
//
// rng=RNG_stdin, seed=unknown
// length= 2 gigabytes (2^31 bytes), time= 29.0 seconds
//   no anomalies in 246 test result(s)
//
// rng=RNG_stdin, seed=unknown
// length= 4 gigabytes (2^32 bytes), time= 57.4 seconds
//   no anomalies in 261 test result(s)
//
// rng=RNG_stdin, seed=unknown
// length= 8 gigabytes (2^33 bytes), time= 115 seconds
//   no anomalies in 274 test result(s)
//
// rng=RNG_stdin, seed=unknown
// length= 16 gigabytes (2^34 bytes), time= 234 seconds
//   no anomalies in 287 test result(s)
//
// rng=RNG_stdin, seed=unknown
// length= 32 gigabytes (2^35 bytes), time= 506 seconds
//   no anomalies in 299 test result(s)
/// 生成下一个 64 位随机数（热路径）。
///
/// 快路径为 `mix64` 搅拌计数器，每 1024 次调用或首次调用时用 `seed` 重播种。
/// 线程安全，可在 `no_std` 下使用。
///
/// # Feature Requirement
///
/// 需要启用 `"rand"` 特性。
///
/// # Examples
///
/// ```rust
/// use lib_unknown::rand::next;
///
/// let a = next();
/// let b = next();
/// assert_ne!(a, b);
/// ```
pub fn next() -> u64 {
    static SEED: AtomicU64 = AtomicU64::new(0);
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let c = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut s = SEED.fetch_add(0x9e3779b97f4a7c15, Ordering::Relaxed);
    if s == 0 || s == 0x9e3779b97f4a7c15 || c.is_multiple_of(1024) {
        s ^= seed();
        SEED.store(s, Ordering::Relaxed);
        s
    } else {
        mix64(s)
    }
}

// // 热路径开销与seed相当
// // RNG = RNG_stdin, seed = unknown
// // test set = core, folding = standard(unknown format)
// //
// // rng=RNG_stdin, seed=unknown
// // length= 32 megabytes (2^25 bytes), time= 2.1 seconds
// //   no anomalies in 163 test result(s)
// //
// // rng=RNG_stdin, seed=unknown
// // length= 64 megabytes (2^26 bytes), time= 4.1 seconds
// //   no anomalies in 174 test result(s)
// //
// // rng=RNG_stdin, seed=unknown
// // length= 128 megabytes (2^27 bytes), time= 7.6 seconds
// //   no anomalies in 187 test result(s)
// //
// // rng=RNG_stdin, seed=unknown
// // length= 256 megabytes (2^28 bytes), time= 14.7 seconds
// //   no anomalies in 201 test result(s)
// //
// // rng=RNG_stdin, seed=unknown
// // length= 512 megabytes (2^29 bytes), time= 28.5 seconds
// //   no anomalies in 216 test result(s)
// //
// // rng=RNG_stdin, seed=unknown
// // length= 1 gigabyte (2^30 bytes), time= 55.7 seconds
// //   no anomalies in 231 test result(s)
// //
// // rng=RNG_stdin, seed=unknown
// // length= 2 gigabytes (2^31 bytes), time= 110 seconds
// //   no anomalies in 246 test result(s)
// pub fn next() -> u64 {
//     static IV: u64 = 0x00001000808c0001;
//
//     // 全局密钥
//     static K1: AtomicU64 = AtomicU64::new(0);
//     static K2: AtomicU64 = AtomicU64::new(0);
//     static INIT_STATE: AtomicUsize = AtomicUsize::new(0);
//
//     // 动态状态源
//     static SEED1: AtomicU64 = AtomicU64::new(0);
//     static SEED2: AtomicU64 = AtomicU64::new(0);
//     static COUNTER: AtomicUsize = AtomicUsize::new(0);
//
//     #[inline]
//     const fn round(x: [u64; 5], c: u64) -> [u64; 5] {
//         let x0 = x[0] ^ x[4];
//         let x2 = x[2] ^ x[1] ^ c;
//         let x4 = x[4] ^ x[3];
//         let tx0 = x0 ^ (!x[1] & x2);
//         let tx1 = x[1] ^ (!x2 & x[3]);
//         let tx2 = x2 ^ (!x[3] & x4);
//         let tx3 = x[3] ^ (!x4 & x0);
//         let tx4 = x4 ^ (!x0 & x[1]);
//         let tx1 = tx1 ^ tx0;
//         let tx3 = tx3 ^ tx2;
//         let tx0 = tx0 ^ tx4;
//         let x0 = tx0 ^ tx0.rotate_right(9);
//         let x1 = tx1 ^ tx1.rotate_right(22);
//         let x2 = tx2 ^ tx2.rotate_right(5);
//         let x3 = tx3 ^ tx3.rotate_right(7);
//         let x4 = tx4 ^ tx4.rotate_right(34);
//         [
//             tx0 ^ x0.rotate_right(19),
//             tx1 ^ x1.rotate_right(39),
//             !(tx2 ^ x2.rotate_right(1)),
//             tx3 ^ x3.rotate_right(10),
//             tx4 ^ x4.rotate_right(7),
//         ]
//     }
//
//     let c = COUNTER.fetch_add(1, Ordering::Relaxed) as u64;
//
//     let mut s1 = SEED1.fetch_add(0x9e3779b97f4a7c15, Ordering::Relaxed);
//     let mut s2 = SEED2.fetch_add(0xbf58476d1ce4e5b9, Ordering::Relaxed);
//
//     if c == 0 || c & 1023 == 0 {
//         let new_entropy = seed();
//         SEED1.fetch_xor(new_entropy, Ordering::Relaxed);
//         s1 ^= new_entropy;
//     } else {
//         s1 = mix64(s1);
//         s2 = mix64(s2);
//     }
//
//     let mut k1 = K1.load(Ordering::Acquire);
//     let mut k2 = K2.load(Ordering::Acquire);
//
//     if k1 == 0 || k2 == 0 {
//         match INIT_STATE.compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed) {
//             Ok(_) => {
//                 k1 = seed();
//                 k2 = seed();
//
//                 if k1 == 0 { k1 = 1; }
//                 if k2 == 0 { k2 = 1; }
//
//                 K1.store(k1, Ordering::Release);
//                 K2.store(k2, Ordering::Release);
//                 INIT_STATE.store(2, Ordering::Release);
//             }
//             Err(_) => {
//                 let mut spins = 0;
//                 while INIT_STATE.load(Ordering::Acquire) != 2 {
//                     core::hint::spin_loop();
//                     spins += 1;
//                     if spins > 10_000 { break; }
//                 }
//
//                 k1 = K1.load(Ordering::Acquire);
//                 k2 = K2.load(Ordering::Acquire);
//                 if k1 == 0 { k1 = c ^ 0xDEADBEEF; }
//                 if k2 == 0 { k2 = s1 ^ 0xCAFEBABE; }
//             }
//         }
//     }
//
//     let mut state = [IV, k1, k2, s1 ^ c, s2];
//
//     let constants = [
//         0xb4, 0xa5, 0x96, 0x87, 0x78, 0x69, 0x5a, 0x4b,
//     ];
//     for &cst in &constants {
//         state = round(state, cst);
//     }
//
//     state[0]
// }

#[cfg(feature = "rand-expand")]
/// 可随机生成的类型。
///
/// # Feature Requirement
///
/// 需要启用 `"rand-expand"` 特性。
///
/// # Examples
///
/// ```rust
/// use lib_unknown::rand::{Random, random};
///
/// let v: u64 = random();
/// let _ = v;
/// ```
pub trait Random {
    fn random() -> Self;
}
#[cfg(feature = "rand-expand")]
/// 生成指定类型的随机值。
///
/// # Feature Requirement
///
/// 需要启用 `"rand-expand"` 特性.
///
/// # Examples
///
/// ```rust
/// use lib_unknown::rand::random;
///
/// let v: u32 = random();
/// let _ = v;
/// ```
#[inline(always)]
pub fn random<T: Random>() -> T {
    T::random()
}
macro_rules! impl_random_int {
    ($($t:ty),*) => {
        $(
            #[cfg(feature = "rand-expand")]
            impl Random for $t {
                #[inline(always)]
                fn random() -> Self {
                    next() as Self
                }
            }
        )*
    };
}
impl_random_int!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize);
#[cfg(feature = "rand-expand")]
impl Random for bool {
    #[inline(always)]
    fn random() -> Self {
        (next() & 1) == 1
    }
}
#[cfg(feature = "rand-expand")]
impl Random for f64 {
    #[inline(always)]
    fn random() -> Self {
        let fraction = next() >> 11;
        fraction as f64 / (1u64 << 53) as f64
    }
}
#[cfg(feature = "rand-expand")]
impl Random for f32 {
    #[inline(always)]
    fn random() -> Self {
        let fraction = (next() as u32) >> 8;
        fraction as f32 / (1u32 << 24) as f32
    }
}

#[cfg(feature = "rand-expand")]
/// 用随机字节填充切片。
///
/// # Feature Requirement
///
/// 需要启用 `"rand-expand"` 特性。
///
/// # Examples
///
/// ```rust
/// use lib_unknown::rand::fill_bytes;
///
/// let mut buf = [0u8; 16];
/// fill_bytes(&mut buf);
/// assert_eq!(buf.len(), 16);
/// ```
pub fn fill_bytes(dest: &mut [u8]) {
    let (chunks, remainder) = dest.as_chunks_mut::<8>();
    for chunk in chunks {
        *chunk = next().to_le_bytes();
    }
    if !remainder.is_empty() {
        let bytes = next().to_le_bytes();
        remainder.copy_from_slice(&bytes[..remainder.len()]);
    }
}

#[cfg(feature = "rand-expand")]
/// 可从区间采样的类型。
///
/// # Feature Requirement
///
/// 需要启用 `"rand-expand"` 特性。
pub trait SampleRange {
    type Output;
    fn sample(self) -> Self::Output;
}

#[cfg(feature = "rand-expand")]
/// 从区间中均匀采样一个随机值，整数用无偏取模实现。
///
/// # Feature Requirement
///
/// 需要启用 `"rand-expand"` 特性。
///
/// # Examples
///
/// ```rust
/// use lib_unknown::rand::random_range;
///
/// let v = random_range(0..100u32);
/// assert!(v < 100);
/// ```
///
/// # Panics
///
/// - 当区间为空（`start >= end`，闭区间为 `start > end`）时发生 panic。
#[inline(always)]
pub fn random_range<R: SampleRange>(range: R) -> R::Output {
    range.sample()
}
macro_rules! impl_sample_range_int {
    ($($t:ty, $u:ty),*) => {
        $(
            // 支持半开区间 Range (..)
            #[cfg(feature = "rand-expand")]
            impl SampleRange for core::ops::Range<$t> {
                type Output = $t;

                #[inline]
                fn sample(self) -> $t {
                    assert!(self.start < self.end, "Invalid range: start must be less than end");

                    let start = self.start as $u;
                    let end = self.end as $u;
                    let span = end.wrapping_sub(start);

                    let threshold = span.wrapping_neg() % span;

                    loop {
                        let r = next() as $u;
                        if r >= threshold {
                            let offset = r % span;
                            return start.wrapping_add(offset) as $t;
                        }
                    }
                }
            }

            // 支持闭区间 RangeInclusive (..=)
            #[cfg(feature = "rand-expand")]
            impl SampleRange for core::ops::RangeInclusive<$t> {
                type Output = $t;

                #[inline]
                fn sample(self) -> $t {
                    let (start_val, end_val) = (*self.start(), *self.end());
                    assert!(start_val <= end_val, "Invalid range: start must be less than or equal to end");

                    if start_val == <$t>::MIN && end_val == <$t>::MAX {
                        return next() as $t;
                    }

                    let start = start_val as $u;
                    let end = end_val as $u;
                    let span = end.wrapping_sub(start).wrapping_add(1);

                    let threshold = span.wrapping_neg() % span;

                    loop {
                        let r = next() as $u;
                        if r >= threshold {
                            let offset = r % span;
                            return start.wrapping_add(offset) as $t;
                        }
                    }
                }
            }
        )*
    };
}

impl_sample_range_int!(
    u8, u8, u16, u16, u32, u32, u64, u64, usize, usize, i8, u8, i16, u16, i32, u32, i64, u64,
    isize, usize
);
macro_rules! impl_sample_range_float {
    ($($t:ty),*) => {
        $(
            #[cfg(feature = "rand-expand")]
            impl SampleRange for core::ops::Range<$t> {
                type Output = $t;
                #[inline]
                fn sample(self) -> $t {
                    assert!(self.start < self.end, "Invalid range: start must be less than end");
                    self.start + random::<$t>() * (self.end - self.start)
                }
            }

            #[cfg(feature = "rand-expand")]
            impl SampleRange for core::ops::RangeInclusive<$t> {
                type Output = $t;
                #[inline]
                fn sample(self) -> $t {
                    let (start, end) = (*self.start(), *self.end());
                    assert!(start <= end, "Invalid range: start must be less than or equal to end");
                    if start == end {
                        return start;
                    }
                    start + random::<$t>() * (end - start)
                }
            }
        )*
    };
}

impl_sample_range_float!(f32, f64);

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[cfg(feature = "std")]
    #[test]
    fn test() {
        for _ in 0..1024 {
            let a = std::time::Instant::now();
            let s = next();
            println!("{:<30}{:?}", s, a.elapsed());
        }
    }
}
