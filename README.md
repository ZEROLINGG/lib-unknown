# lib-unknown

> **一个神秘的共享库**

[![Crates.io](https://img.shields.io/crates/v/lib-unknown.svg)](https://crates.io/crates/lib-unknown)
[![Downloads](https://img.shields.io/crates/d/lib-unknown.svg)](https://crates.io/crates/lib-unknown)
[![Documentation](https://docs.rs/lib-unknown/badge.svg)](https://docs.rs/lib-unknown)
[![License](https://img.shields.io/crates/l/lib-unknown.svg)](#开源协议)


零依赖（`[dependencies]` 为空）、默认 `no_std` 的系统编程基础库，提供五个模块：硬件时间戳熵源与随机数（`rand`）、跨平台裸系统调用与动态符号解析（`sys`）、用后清零的定容内存类型（`types`）、实验性搅拌与流密码原语（`crypto`）、开发期动态编译运行 harness（`dyntest`，需 `std`）。

---

## 目录

- [设计哲学](#设计哲学)
- [快速开始](#快速开始)
- [API 预览](#api-预览)
- [适用场景 vs 不适用场景](#适用场景-vs-不适用场景)
- [特性标志](#特性标志)
- [平台与环境支持](#平台与环境支持)
- [最小 Rust 版本](#最小-rust-版本)
- [安全性](#安全性)
- [贡献](#贡献)
- [变更日志](#变更日志)
- [开源协议](#开源协议)

---

## 设计哲学

### 核心原则

1. **零依赖** —— `[dependencies]` 为空（`wasm32` 下可选 `wasm-bindgen`），不链接 libc，符号解析与系统调用全部手写 `asm!` / 动态解析。
2. **`no_std` 默认** —— 默认即无标准库，堆内存类型需显式开启 `alloc`，`dyntest` 等开发工具才需要 `std`。
3. **不安全显式化** —— 所有裸调用与原始指针操作收敛在 `unsafe` 边界内，公开 `unsafe fn` 均标注 `# Safety`，内部 `unsafe` 块附 `SAFETY` 注释。

### 权衡取舍 (Trade-offs)

| 我们选择了 | 而不是 | 原因 |
| :--- | :--- | :--- |
| 自研搅拌/流密码 | 成熟审计算法 | 零依赖与体积优先；已在文档中明确标注实验性质，不可直接用于生产密码学场景 |
| 裸 `syscall` + 动态解析 | 链接 libc | 适配无 libc / 嵌入式环境；代价是不支持的目标组合编译期直接报错 |

### 非目标 (Non-Goals)

- 不追求密码学合规审计与 CSPRNG 认证，熵质量数据（NIST SP800-90B / PractRand）仅为作者自测记录。
- 不做上层业务抽象，只提供可组合的底层原语。

---

## 快速开始

```toml
[dependencies]
lib-unknown = "0.1.2"
```

运行此示例需启用 `rand` 特性（默认已启用）。

```rust
# #[cfg(feature = "rand")]
# {
use lib_unknown::rand::{next, fill_bytes};

let x = next();
let mut buf = [0u8; 16];
fill_bytes(&mut buf);
assert_ne!(x, 0);
# }
```

## API 预览

以下是本库核心 API 的简化概览。完整签名（含泛型约束、特性门控）请以 [docs.rs](https://docs.rs/lib-unknown) 为准。

### 模块结构

```text
lib_unknown
├── rand       — probe/seed/next/shuffle/random/fill_bytes/random_range
├── sys        — syscall::syscall0~6，unix::resolve/sys_*，win::resolve
├── types      — bytes::StackBytes/HeapBytes，str::StackStr/HeapStr
├── crypto     — base::mix64/mse/imse（实验性质）
└── dyntest    — DnyRun/BatchRunner（std-only，开发期用）
```

### 核心函数（节选）

```text
rand::probe() -> u64            // 硬件时间戳熵源
rand::seed() -> u64             // 混合重播种（勿用于热路径）
rand::next() -> u64             // 热路径输出，每 1024 次重播种
sys::unix::syscall::sys_read(fd, buf) -> SysResult
types::bytes::StackBytes<N>     // 定容字节容器，用后清零
crypto::base::mix64(x) -> u64   // 雪崩混合（内部搅拌用）
```

---

## 适用场景 vs 不适用场景

**适合：**
- 无 libc / 嵌入式 / 裸机环境需要熵源与系统调用
- 短密钥、令牌等敏感数据的定容内存处理
- 开发期需要动态编译运行代码片段的测试 harness

**不适合：**
- 需要合规审计的密码学与随机数场景（请使用 `rand` / `getrandom` 等审计过的 crate）
- 通用应用开发（本库只提供底层原语，无上层抽象）

---

## 特性标志

| Feature | 默认 | 说明 |
|---|---|---|
| `default` | ✅ | `rand` + `rand-expand` + `sys` + `types` + `crypto` |
| `rand` / `rand-expand` | ✅ | 熵源 `probe/seed/next`、区间采样与 `fill_bytes` |
| `sys` / `sys-syscall` / `sys-unix` / `sys-win` | ✅ | 裸 `syscall0~6`、常量封装、动态符号解析 |
| `types` / `types-bytes` / `types-str` | ✅ | `StackBytes/HeapBytes/StackStr/HeapStr`（用后清零） |
| `crypto` | ✅ | `mix64`、`mse/imse` 流密码（实验性质，非审计算法） |
| `std` / `alloc` | ❌ | `std = ["alloc"]`，开启堆内存类型 |
| `rand-safe-stack` | ❌ | `seed` 使用独立栈缓冲代替复用调用栈 |
| `dyntest` | ❌ | 需 `std`，动态编译运行 harness |

## 平台与环境支持

- **操作系统**：Linux / Android / macOS / Windows（`syscall` 按 target 分发，不支持目标编译期报错）
- **`no_std` 支持**：是（默认即 `no_std`，`#![cfg_attr(not(feature = "std"), no_std)]`）
- **Unsafe 代码**：含 `unsafe`（裸 syscall、`asm!` 采样、原始指针拷贝），公开 `unsafe fn` 均已标注 `# Safety`

## 最小 Rust 版本

MSRV 未在 `Cargo.toml` 声明，在 `rustc 1.98.1` 测试稳定。

## 安全性

- 本库含 `unsafe`（裸 syscall、`asm!` 采样、原始指针拷贝），用途见[平台与环境支持](#平台与环境支持)；调用 `unsafe` 接口前请阅读其 `# Safety`。
- `crypto` 为未经审计的自研原语，仅供内部搅拌与实验，请勿用于生产密码学场景。
- `dyntest` 会编译并执行传入的代码，仅可在隔离沙箱中使用，不可用于生产。

---

## 贡献

欢迎提交 Issue 和 Pull Request！

提交 PR 前请先阅读[设计哲学](#设计哲学)，与项目核心原则冲突的功能建议可能不会被采纳（欢迎在 Issue 中先讨论）。

---

## 变更日志

详见 [CHANGELOG.md](CHANGELOG.md)。

## 开源协议

[MIT License](./License)
