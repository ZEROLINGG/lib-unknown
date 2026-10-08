# lib-unknown

> **专为极简环境与安全研究设计的高隐蔽性底层 Rust 原语库**

[![Crates.io](https://img.shields.io/crates/v/lib-unknown.svg)](https://crates.io/crates/lib-unknown)
[![Downloads](https://img.shields.io/crates/d/lib-unknown.svg)](https://crates.io/crates/lib-unknown)
[![Documentation](https://docs.rs/lib-unknown/badge.svg)](https://docs.rs/lib-unknown)
[![License](https://img.shields.io/crates/l/lib-unknown.svg)](#开源协议)

`lib-unknown` 是一个纯 Rust 实现的 `no_std` 无依赖底层工具库。它脱离了对标准库及 libc 的依赖，旨在为**对抗性安全评估框架、控制客户端及高隐秘性代码执行环境**提供无符号特征、抗分析、轻量化的核心底层支撑。

---

## 目录

- [核心能力](#核心能力)
- [快速开始](#快速开始)
- [API 预览与模块设计](#api-预览与模块设计)
- [典型应用场景](#典型应用场景)
- [特性标志](#特性标志)
- [平台与环境支持](#平台与环境支持)

---

## 核心能力

| 模块 | 表面功能 |
| :--- | :--- |
| **`sys`** | 裸 Syscall & 动态 API 解析 |
| **`rand`** | 硬件时间戳熵源 |
| **`crypto`** | 实验性流密码与数据搅拌 |
| **`types`** | 自清零定容 Stack/Heap 容器 |
| **`dyntest`** | 动态编译与内存运行 Harness |

---

## 快速开始

在 Cargo.toml 中引入：

```toml
[dependencies]
lib-unknown = { version = "0.1", default-features = false, features = ["sys", "rand", "rand-expand", "types"] }
```

### 示例

运行此示例需启用 `rand` 与 `types` 特性（默认已启用）。

```rust
#[cfg(all(feature = "rand", feature = "types"))]
fn main() {
    use lib_unknown::rand::{probe, seed, next, fill_bytes, random_range};
    use lib_unknown::types::bytes::StackBytes;

    // 1. 硬件时间戳探针
    let t1 = probe();
    let t2 = probe();
    let delta = t2.wrapping_sub(t1);

    // 2. 混合多熵源种子生成
    let entropy_seed = seed();

    // 3. 热路径随机数生成 
    let random_val = next();

    // 4. 区间采样
    let sleep_ms = random_range(1000u32..5000u32);

    // 5. 批量填充字节
    let mut key_buf = [0u8; 32];
    fill_bytes(&mut key_buf);
}

// 缺特性时以空 main 兜底，保证 --no-default-features 下 doctest 可编译。
#[cfg(not(all(feature = "rand", feature = "types")))]
fn main() {}
```

---

## API 预览与模块设计

```text
lib_unknown
├── sys       — 裸 syscall0~6，PEB/EAT 动态导出表解析，绕过 Hook 的底层调用
├── rand      — 硬件微架构 (RDTSC) 探针，无 OS API 依赖的熵源生成与环境感知
├── crypto    — 非标准混淆与实验性雪崩搅拌，用于消除数据静态特征
├── types     — 栈/堆上防转储容器 (StackBytes/StackStr/StackCStr)， Drop 时强行覆写擦除
│               其中 `cstr` 为 NUL 结尾 C 字符串（允许非 UTF-8），可直喂 `sys_open` 等调用
└── dyntest   — (std-only) 动态代码块编译与内存执行 Harness，用于验证内存加载器
```

---

## 典型应用场景

### 适用场景
- **红队工具开发**：需要体积极小、无第三方库依赖、静态特征极低的轻量级载荷。
- **反逆向与高强度保护**：在不使用重量级 VM 加壳的前提下，提供隐秘的 API 解析、对抗性时序校验和内存清理。
- **裸机 / 无 OS 评估环境**：无法依赖 libc 或标准操作系统服务的嵌入式/安全测试场景。


---

## 特性标志

本库采用高度模块化的 Feature 切分，允许按需引入以实现**最小化二进制体积（Bloat-free）**：

| Feature         | 说明                                       |
|-----------------|------------------------------------------|
| `sys`           | 启用直接系统调用与动态符号解析                          |
| `rand`          | 启用硬件级微架构熵源生成器                            |
| `types`         | 启用用后即焚的内存安全容器（`StackBytes` / `StackStr` / `StackCStr`） |
| `types-cstr`    | 启用 NUL 结尾 C 字符串容器（`StackCStr` / `HeapCStr`，允许非 UTF-8） |
| `crypto`        | 启用用于特征消除的混淆与流密码原语                        |
| `std` / `alloc` | 显式开启堆支持                                  |

---

## 平台与环境支持

- **架构/系统**：x86_64 / AArch64 (Linux, Windows, macOS, Android)
- **运行环境**：完全支持 `no_std`，可直接作为独立 Payload 或 Reflective DLL 的核心依赖。
- **编译时安全性**：所有底层裸指针及内联汇编均封装在显式的 `unsafe` 隔离层中，确保上层调用的内存安全。

---

### 开源协议

本项目基于 [MIT License](./License) 协议开源。