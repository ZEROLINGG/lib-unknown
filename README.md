# lib-unknown

> **一个神秘的共享库**

`no_std` 兼容的底层基础库：硬件熵 RNG、自研密码原语、裸系统调用、定容安全内存类型。

## 快速开始

```toml
[dependencies]
lib-unknown = "0.1.0"
```

```rust
use lib_unknown::rand::{next, fill_bytes};

let x = next();
let mut buf = [0u8; 16];
fill_bytes(&mut buf);
assert_ne!(x, 0);
```

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

MSRV 未在 `Cargo.toml` 声明，本次体检使用 `rustc 1.98.1`。

## 变更日志

详见 [CHANGELOG.md](CHANGELOG.md)。

## 开源协议

[MIT License](License)
