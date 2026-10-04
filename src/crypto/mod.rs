//! 自研密码原语（实验性质，非审计算法）：SBox 混合（`mix64`）、轻量哈希与 `mse`/`imse` 流密码（`base`）。
//!
//! 需要启用 `"crypto"` 特性。
pub mod base;
