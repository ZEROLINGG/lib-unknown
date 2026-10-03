# Changelog

本文件记录本项目所有值得关注的变更。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
版本号遵循 [语义化版本 2.0.0](https://semver.org/lang/zh-CN/)。

> **版本号说明（0.x 阶段）**：在 1.0.0 发布之前，次版本号（`0.MINOR.0`）的变更
> 也可能包含破坏性改动，请留意标注为 **[BREAKING]** 的条目。

---

## [未发布] (Unreleased)

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
