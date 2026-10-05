# Changelog

本文件记录本项目所有值得关注的变更。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
版本号遵循 [语义化版本 2.0.0](https://semver.org/lang/zh-CN/)。

> **版本号说明（0.x 阶段）**：在 1.0.0 发布之前，次版本号（`0.MINOR.0`）的变更
> 也可能包含破坏性改动，请留意标注为 **[BREAKING]** 的条目。

---

## [未发布] (Unreleased)

### 新增 (Added)

-

### 变更 (Changed)

-

### 修复 (Fixed)

-

---

## [0.1.5] - 2026-10-05

### 新增 (Added)

- `dyntest`: 新增 `DnyRun::bin_path()`，返回当前配置下产物可执行文件的预期路径（考虑 `target-dir` 配置、`is_release` 与平台后缀），`run_no_build()` 改为复用它
- `dyntest`: 新增 `DnyRun::cargo(&[args], timeout)` 通用入口，原样透传 `cargo <args>`（`check` / `test` / `clippy` 等），耗时记入 `run_duration`，`build_duration` 恒为零
- `dyntest`: `SystemError` 新增 `CargoFailed` 变体，`cargo()` 正常退出但退出码非零时置该错误并保留 `stdout` / `stderr` / `exit_code` 供诊断；`DnyResult` 展示新增 `CARGO ERROR (CE)` 分支

### 变更 (Changed)

- `dyntest`: `SystemError` 加 `#[non_exhaustive]`，后续新增变体不再是对下游穷举匹配的破坏性变更；`CompileFailed` 文档明确为退出码非零
- `dyntest`: `cargo()` 明确**不透传 `is_release`**，与 `build()` 自动追加 `--release` 不同，需要 release 行为时由调用者显式传入（如 `&["test", "--release"]`）
- `rand` / `dyntest` 公开 API 文档补齐：4 处 `unsafe` 块补 `SAFETY` 注释、`Random` / `SampleRange` 取值语义与示例、`shuffle` 算法说明；零行为变更

### 修复 (Fixed)

- 恢复 README 快速开始示例的特性门控（隐藏 `cfg(all(feature = "rand", feature = "types"))` 包裹，缺特性时以空 `main` 兜底），修复 `cargo test --doc --no-default-features` 失败；示例前注明所需特性（默认已启用）

---

## [0.1.4] - 2026-10-05

### 变更 (Changed)

- 仅版本号提升（`0.1.3` → `0.1.4`），无功能、文档与依赖变更

---

## [0.1.3] - 2026-10-05

### 变更 (Changed)

- README 全面重写：简介改为高隐蔽性底层原语库定位；章节精简（移除设计哲学、权衡取舍、非目标、MSRV、安全性、贡献、变更日志、开源协议小节，目录同步）；依赖声明改为 `default-features = false` + 按需 `features` 写法；示例改用 `probe` / `seed` / `next` / `random_range` / `fill_bytes` 完整链路
- README 依赖示例版本号由 `0.1.2` 改为 `0.1` 兼容区间
- `crypto`：`mix64` 文档改为"带 SBox 头尾扰动和前馈的不可逆 64 位雪崩混合函数"；

---

## [0.1.2] - 2026-10-04

### 修复 (Fixed)

- 补 `#![cfg_attr(docsrs, feature(doc_cfg))]`，否则 docs.rs（`--cfg docsrs`）构建因实验性 `doc(cfg)` 属性失败；stable 工具链不受影响。已用与 docs.rs 相同版本 nightly（1.101.0）复现验证通过

## [0.1.1] - 2026-10-04

### 新增 (Added)

- 为 `sys/unix/syscall` 的 20 个 `sys_*` 封装、`SysResult` / `SysErr` / `ErrorMessage`（含 `as_str`）及各平台 `constants` / `flags` / `ptrace_req` 模块补齐 rustdoc，新增 23 个 doctest；残余单常量摘要已更新 `TODO_DOCS.md`
- 为全部 12 个模块文件补 `//!` 模块头文档；为 `dyntest` 全公有 API（`SystemError` / `DnyResult` / `DnyTask` / `BatchRunner` / `dny_run*` / `DnyRun` / `DynTestLock` / `clear_dny_project`，含公有字段与 `# Panics`）及 `types` 全公有 API（`Bytes` / `Str` trait、`StackBytes` / `HeapBytes` / `StackStr` / `HeapStr`、`BytesError` / `StrError`，含 `# Errors` / `# Panics`）补齐 rustdoc，新增 38 个 doctest；`TODO_DOCS.md` 删除已完成 7 行，残余 4 行
- `Cargo.toml` 新增 `[package.metadata.docs.rs]`（`all-features = true`）；公开模块加 `#[cfg_attr(docsrs, doc(cfg(feature = "...")))]`
- crate 根改用 `#![doc = include_str!("../README.md")]`，README 为唯一事实源

### 变更 (Changed)

- `README` 快速开始示例注明需 `rand` 特性，并以隐藏 `#[cfg(feature = "rand")]` 包裹以兼容 `--no-default-features` 下的 doctest
- `README` 中 `[MIT License](License)` 改为 `./License`，消除 rustdoc 断链警告
- `README` 按标准模板补齐徽章（无 CI 工作流故略去 CI 徽章）、综合简介、目录、设计哲学、API 预览、适用场景、安全性、贡献小节（无 `benches/` 故略去性能章；单语言故无语言链接）

### 修复 (Fixed)

- `.gitignore` 补齐标准 Rust 构建产物、密钥、IDE、操作系统条目（保留原有自定义内容）
- 为 `unix::resolve`、`volatile_zero`、`zeroed_box`、`shuffle_with` 及 `str` 宏内两处 `unsafe` 块补 `SAFETY` 注释
- `cargo fmt --check` 通过；`cargo clippy --lib --all-features -- -D warnings` 通过；`cargo test --all-features`（20 单元 + 73 doctest）与 `--no-default-features` doctest 通过；`cargo doc` 无警告（含 `broken_intra_doc_links`）；`i686-unknown-linux-gnu` 检查通过
- 修复 32 位 x86 专属 `sys_mmap` 分支的 `SysErr::Arg("...".to_string())` 类型错误，改为直接构造 `ErrorMessage`（截断语义与 `FromStr` 一致）；新增 `i686-unknown-linux-gnu` 工具链验证通过
- 修复 x86 下 `syscall4/5/6` 内联汇编显式使用 `esi` 被 LLVM 拒绝的问题，改用 `push/mov/pop esi` 传递（与既有 `ebp` 处理一致）；修复 `rand::reg_sig` x86 分支数字标签 `1:` 触发的 `binary_asm_labels` 错误，改用 `2:` 编号

---

## [0.1.0] - 2026-10-03

### 新增 (Added)

- 项目初始版本：`rand` / `sys` / `types` / `crypto` / `dyntest` 五大模块
- 为 20 项核心公开 API 补齐 rustdoc（含 `rand::probe/seed/next/shuffle/random/fill_bytes/random_range`、`syscall0~6`、`unix/win::resolve`、`mix64/mix64_next`、`from_raw_parts_mut`），新增 12 个 doctest
- 新增 `README.md`（简体中文最小版）
- `Cargo.toml` 元数据：`description`、`license = "MIT"`、`readme`、`repository`、`exclude = ["TODO_DOCS.md"]`
- `BatchRunner` 新增 `Default` 实现
- `ErrorMessage` 实现标准 `core::str::FromStr`（`Err = Infallible`，截断语义不变，返回 `Result`）；用法：`s.parse::<ErrorMessage>().unwrap()`

### 变更 (Changed)

- `fill_bytes` 内部由 `chunks_exact_mut(8)` 改为 `as_chunks_mut::<8>()`，行为不变
- `next` 的取模判断改为 `c.is_multiple_of(1024)`，`as_sys_result` 改为 `(-4095..0).contains(&ret)`，`PartialOrd` 改为 `Some(self.cmp(other))` 规范实现
- `StackBytes` 掩码常量 `STACK_MASK` 由 `u16` 改为 `usize`（内部实现细节）
- 包名由 `lib` 改为 `lib-unknown`，文档示例同步为 `use lib_unknown::...`

### 修复 (Fixed)

- 修复启用 `rand-safe-stack` 时编译失败：`[u8; STACK_MASK]` 数组长度必须为 `usize`；`sp` 因多余分号被推断为 `()` 导致 `sp.add` 不存在
- 修复 `p8` 的 `needless_range_loop`、测试模块 `unused_imports/unused_variables` 警告
- 修复包名改名后 12 个 doctest 的 `use lib::` 导入失效
- `cargo fmt --check` 通过；`cargo clippy --lib --all-features -- -D warnings` 通过
- 说明：首发即含上述 API 形态，不单独保留破坏性迁移条目；剩余 400+ 公开项文档见内部 `TODO_DOCS.md`（已 `exclude`，不随包发布）

---

## [0.1.0] - 2026-10-03

### 新增 (Added)

- 项目初始版本：`rand` / `sys` / `types` / `crypto` / `dyntest` 五大模块
