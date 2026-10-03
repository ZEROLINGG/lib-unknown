#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(feature = "rand")]
pub mod rand;
#[cfg(any(
    feature = "sys",
    feature = "sys-syscall",
    feature = "sys-win",
    feature = "sys-unix"
))]
pub mod sys;

#[cfg(any(feature = "types", feature = "types-str", feature = "types-bytes"))]
pub mod types;

#[cfg(feature = "crypto")]
pub mod crypto;
#[cfg(all(feature = "dyntest", feature = "std"))]
pub mod dyntest;

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
