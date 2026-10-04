#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]
// docs.rs 以 `--cfg docsrs` 构建时才启用 `doc(cfg)`，stable 工具链不受影响。
#![cfg_attr(docsrs, feature(doc_cfg))]
#[cfg(feature = "rand")]
#[cfg_attr(docsrs, doc(cfg(feature = "rand")))]
pub mod rand;
#[cfg(any(
    feature = "sys",
    feature = "sys-syscall",
    feature = "sys-win",
    feature = "sys-unix"
))]
#[cfg_attr(docsrs, doc(cfg(feature = "sys")))]
pub mod sys;

#[cfg(any(feature = "types", feature = "types-str", feature = "types-bytes"))]
#[cfg_attr(docsrs, doc(cfg(feature = "types")))]
pub mod types;

#[cfg(feature = "crypto")]
#[cfg_attr(docsrs, doc(cfg(feature = "crypto")))]
pub mod crypto;
#[cfg(all(feature = "dyntest", feature = "std"))]
#[cfg_attr(docsrs, doc(cfg(feature = "dyntest")))]
pub mod dyntest;

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
