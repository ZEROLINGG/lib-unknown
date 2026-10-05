//! 自研密码原语实现：SBox 表、雪崩混合函数、轻量流式哈希与 `mse`/`imse` 加解密。
//!
//! 需要启用 `"crypto"` 特性。
#![allow(unused)]
#![no_std]
pub static SBOX_BASE: [[u8; 256]; 2] = [
    // SBOX 0
    [
        0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab,
        0x76, 0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4,
        0x72, 0xc0, 0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71,
        0xd8, 0x31, 0x15, 0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2,
        0xeb, 0x27, 0xb2, 0x75, 0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6,
        0xb3, 0x29, 0xe3, 0x2f, 0x84, 0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb,
        0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf, 0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45,
        0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8, 0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5,
        0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2, 0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44,
        0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73, 0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a,
        0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb, 0xe0, 0x32, 0x3a, 0x0a, 0x49,
        0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79, 0xe7, 0xc8, 0x37, 0x6d,
        0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08, 0xba, 0x78, 0x25,
        0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a, 0x70, 0x3e,
        0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e, 0xe1,
        0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
        0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb,
        0x16,
    ],
    // SBOX 1
    [
        0x52, 0x09, 0x6a, 0xd5, 0x30, 0x36, 0xa5, 0x38, 0xbf, 0x40, 0xa3, 0x9e, 0x81, 0xf3, 0xd7,
        0xfb, 0x7c, 0xe3, 0x39, 0x82, 0x9b, 0x2f, 0xff, 0x87, 0x34, 0x8e, 0x43, 0x44, 0xc4, 0xde,
        0xe9, 0xcb, 0x54, 0x7b, 0x94, 0x32, 0xa6, 0xc2, 0x23, 0x3d, 0xee, 0x4c, 0x95, 0x0b, 0x42,
        0xfa, 0xc3, 0x4e, 0x08, 0x2e, 0xa1, 0x66, 0x28, 0xd9, 0x24, 0xb2, 0x76, 0x5b, 0xa2, 0x49,
        0x6d, 0x8b, 0xd1, 0x25, 0x72, 0xf8, 0xf6, 0x64, 0x86, 0x68, 0x98, 0x16, 0xd4, 0xa4, 0x5c,
        0xcc, 0x5d, 0x65, 0xb6, 0x92, 0x6c, 0x70, 0x48, 0x50, 0xfd, 0xed, 0xb9, 0xda, 0x5e, 0x15,
        0x46, 0x57, 0xa7, 0x8d, 0x9d, 0x84, 0x90, 0xd8, 0xab, 0x00, 0x8c, 0xbc, 0xd3, 0x0a, 0xf7,
        0xe4, 0x58, 0x05, 0xb8, 0xb3, 0x45, 0x06, 0xd0, 0x2c, 0x1e, 0x8f, 0xca, 0x3f, 0x0f, 0x02,
        0xc1, 0xaf, 0xbd, 0x03, 0x01, 0x13, 0x8a, 0x6b, 0x3a, 0x91, 0x11, 0x41, 0x4f, 0x67, 0xdc,
        0xea, 0x97, 0xf2, 0xcf, 0xce, 0xf0, 0xb4, 0xe6, 0x73, 0x96, 0xac, 0x74, 0x22, 0xe7, 0xad,
        0x35, 0x85, 0xe2, 0xf9, 0x37, 0xe8, 0x1c, 0x75, 0xdf, 0x6e, 0x47, 0xf1, 0x1a, 0x71, 0x1d,
        0x29, 0xc5, 0x89, 0x6f, 0xb7, 0x62, 0x0e, 0xaa, 0x18, 0xbe, 0x1b, 0xfc, 0x56, 0x3e, 0x4b,
        0xc6, 0xd2, 0x79, 0x20, 0x9a, 0xdb, 0xc0, 0xfe, 0x78, 0xcd, 0x5a, 0xf4, 0x1f, 0xdd, 0xa8,
        0x33, 0x88, 0x07, 0xc7, 0x31, 0xb1, 0x12, 0x10, 0x59, 0x27, 0x80, 0xec, 0x5f, 0x60, 0x51,
        0x7f, 0xa9, 0x19, 0xb5, 0x4a, 0x0d, 0x2d, 0xe5, 0x7a, 0x9f, 0x93, 0xc9, 0x9c, 0xef, 0xa0,
        0xe0, 0x3b, 0x4d, 0xae, 0x2a, 0xf5, 0xb0, 0xc8, 0xeb, 0xbb, 0x3c, 0x83, 0x53, 0x99, 0x61,
        0x17, 0x2b, 0x04, 0x7e, 0xba, 0x77, 0xd6, 0x26, 0xe1, 0x69, 0x14, 0x63, 0x55, 0x21, 0x0c,
        0x7d,
    ],
];

/// 步进一次并混合，`mix64(x + GOLDEN)` 的便捷封装。
///
/// # Feature Requirement
///
/// 需要启用 `"crypto"` 特性。
///
/// # Examples
///
/// ```rust
/// use lib_unknown::crypto::base::mix64_next;
///
/// let y = mix64_next(0);
/// assert_ne!(y, 0);
/// ```
pub fn mix64_next(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e3779b97f4a7c15);
    mix64(x)
}

/// 带 SBox 头尾扰动和前馈的不可逆 64 位雪崩混合函数。
///
///
/// # Feature Requirement
///
/// 需要启用 `"crypto"` 特性。
///
/// # Examples
///
/// ```rust
/// use lib_unknown::crypto::base::mix64;
///
/// assert_ne!(mix64(0), mix64(1));
/// ```
pub fn mix64(mut x: u64) -> u64 {
    let o1 = x;
    let s1 = s8(x as u8, &SBOX_BASE[0]); // s-box
    let s2 = s8((x >> 56) as u8, &SBOX_BASE[0]);
    x ^= s1 as u64;
    x ^= (s2 as u64) << 56;

    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    let o2 = x;
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    x = x ^ (x >> 31);

    x = x.wrapping_add(o2);
    x ^= o1;

    x
}

pub struct LightHasher {
    hash: u64,
    buf: [u8; 8],
    buf_len: u8, // 0..=7
}

impl LightHasher {
    #[inline(always)]
    pub fn new(seed: u64) -> Self {
        Self {
            hash: seed,
            buf: [0; 8],
            buf_len: 0,
        }
    }

    #[inline(always)]
    fn next_input(hash: u64, i: u64) -> u64 {
        let i = i.wrapping_mul(0x517cc1b727220a95).wrapping_pow(2);
        hash.rotate_left(5) ^ i
    }

    #[inline(always)]
    fn add_to_hash(&mut self, i: u64) {
        self.hash = Self::next_input(self.hash, i);
    }

    #[inline(always)]
    fn write_bytes(&mut self, mut bytes: &[u8]) {
        let buf_len = self.buf_len as usize;
        if buf_len > 0 {
            let take = (8 - buf_len).min(bytes.len());
            self.buf[buf_len..buf_len + take].copy_from_slice(&bytes[..take]);
            self.buf_len = (buf_len + take) as u8;
            bytes = &bytes[take..];

            if self.buf_len == 8 {
                let chunk = u64::from_le_bytes(self.buf);
                self.add_to_hash(chunk);
                self.buf_len = 0;
            }
        }

        while bytes.len() >= 8 {
            let chunk = u64::from_le_bytes(bytes[..8].try_into().unwrap());
            self.add_to_hash(chunk);
            bytes = &bytes[8..];
        }

        if !bytes.is_empty() {
            self.buf[..bytes.len()].copy_from_slice(bytes);
            self.buf_len = bytes.len() as u8;
        }
    }
}

impl core::hash::Hasher for LightHasher {
    #[inline(always)]
    fn finish(&self) -> u64 {
        let mut hash = self.hash;

        let mut remaining = [0_u8; 8];
        let len = self.buf_len as usize;
        remaining[..len].copy_from_slice(&self.buf[..len]);

        hash = Self::next_input(hash, u64::from_le_bytes(remaining));
        mix64(hash)
    }

    #[inline(always)]
    fn write(&mut self, bytes: &[u8]) {
        self.write_bytes(bytes);
    }

    #[inline(always)]
    fn write_u8(&mut self, i: u8) {
        self.write_bytes(&[i]);
    }

    #[inline(always)]
    fn write_u32(&mut self, i: u32) {
        self.write_bytes(&i.to_le_bytes());
    }

    #[inline(always)]
    fn write_u64(&mut self, i: u64) {
        if self.buf_len == 0 {
            self.add_to_hash(i);
        } else {
            self.write_bytes(&i.to_le_bytes());
        }
    }

    #[inline(always)]
    fn write_usize(&mut self, i: usize) {
        self.write_u64(i as u64);
    }
}

pub fn derive<T: core::hash::Hash>(value: T) -> u64 {
    let mut hasher = LightHasher::new(u32::MAX as u64);
    use core::hash::Hasher;
    value.hash(&mut hasher);
    hasher.finish()
}
#[inline(never)]
pub fn shuffle_with<T, U>(mut table: U, mut seed: u64) -> U
where
    U: AsMut<[T]>,
{
    let slice = table.as_mut();
    let len = slice.len();
    if len <= 1 {
        return table;
    }

    let ptr = slice.as_mut_ptr();

    for i in (1..len).rev() {
        let bound = (i + 1) as u64;

        seed = mix64_next(seed);
        let mut m = (seed as u128) * (bound as u128);
        let mut l = m as u64;

        if l < bound {
            let t = bound.wrapping_neg() % bound;
            while l < t {
                seed = mix64_next(seed);
                m = (seed as u128) * (bound as u128);
                l = m as u64;
            }
        }

        let j = (m >> 64) as usize;
        // SAFETY: `ptr` 来自非空切片，`i/j < len`（`j` 为 Lemire 无偏约减结果，恒小于 `bound = i+1 <= len`），
        // 两指针均在同一分配内且 `T` 可移动，调用期间无其他借用。
        unsafe {
            core::ptr::swap(ptr.add(i), ptr.add(j));
        }
    }
    table
}

#[inline]
pub fn p8(x: u8, s: &[u8]) -> u8 {
    assert_eq!(s.len(), 8);
    let mut out = 0u8;
    for (dst, &src) in s.iter().enumerate().take(8) {
        assert!(src < 8);
        let bit = (x >> src) & 1;
        out |= bit << dst;
    }
    out
}

#[inline]
pub fn s8(x: u8, s: &[u8]) -> u8 {
    assert_eq!(s.len(), 256);
    s[x as usize]
}

// pub fn s64(x: u64, s: &[u8]) -> u64 {
//     assert_eq!(s.len(), 256);
//     let mut bytes = x.to_le_bytes();
//     for b in &mut bytes {
//         *b = s[*b as usize];
//     }
//     u64::from_le_bytes(bytes)
// }
#[inline(never)]
pub fn inv_p(s: &[u8]) -> Option<[u8; 8]> {
    if s.len() != 8 {
        return None;
    }

    let mut inv = [0u8; 8];
    let mut seen = [false; 8];

    for (dst, &src) in s.iter().enumerate() {
        if src >= 8 {
            return None;
        }
        if seen[src as usize] {
            return None;
        }
        seen[src as usize] = true;
        inv[src as usize] = dst as u8;
    }
    Some(inv)
}
#[inline(never)]
pub fn inv_s(s: &[u8]) -> Option<[u8; 256]> {
    if s.len() != 256 {
        return None;
    }

    let mut inv = [0u8; 256];
    let mut seen = [false; 256];

    for (i, &val) in s.iter().enumerate() {
        if seen[val as usize] {
            return None;
        }
        seen[val as usize] = true;
        inv[val as usize] = i as u8;
    }

    Some(inv)
}

pub fn gen_p(seed: u64) -> [u8; 8] {
    shuffle_with([0, 1, 2, 3, 4, 5, 6, 7], seed)
}

pub fn gen_s(seed: u64) -> [u8; 256] {
    shuffle_with(core::array::from_fn(|i| i as u8), seed)
}

#[inline]
pub fn inv_mul8(m: u8) -> u8 {
    debug_assert!(m & 1 == 1, "m 必须是奇数才有逆元");
    let mut inv = m; // 对 mod 2 成立
    inv = inv.wrapping_mul(2u8.wrapping_sub(m.wrapping_mul(inv))); // 2 bits
    inv = inv.wrapping_mul(2u8.wrapping_sub(m.wrapping_mul(inv))); // 4 bits
    inv = inv.wrapping_mul(2u8.wrapping_sub(m.wrapping_mul(inv))); // 8 bits
    inv
}

/// x: 明文字节
/// l: 上一字节对应的密文
/// i: 当前字节的序号/轮数
/// k: 密钥
/// p: 当前轮使用的 8 元素比特置换表
/// s: 当前轮使用的 256 元素字节替换表
#[inline(never)]
pub fn r8<K: Into<u128>>(
    mut x: u8,
    l: u8,
    i: usize,
    k: K,
    mut p: Option<&mut [u8]>,
    mut s: Option<&mut [u8]>,
) -> u8 {
    let k = k.into();
    let ki = k.rotate_right(i as u32) as u8;
    let r = mix64(
        k.rotate_right(l as u32) as u64 ^ (l as u64).rotate_left(17) ^ (i as u64).rotate_left(31),
    );
    x ^= l ^ !(i as u8) ^ ki ^ r as u8;

    if let Some(p_box) = &mut p {
        x = p8(x, p_box);

        let val1 = (r & 0b111) as u8;
        let val2 = ((r >> 3) & 0b111) as u8;
        let pos1 = p_box.iter().position(|&v| v == val1).unwrap();
        let pos2 = p_box.iter().position(|&v| v == val2).unwrap();
        p_box.swap(pos1, pos2);
    }

    x = x.wrapping_mul(inv_mul8(ki | 1));
    x = x.wrapping_mul(inv_mul8(mix64_next(r) as u8 | 1));

    if let Some(s_box) = &mut s {
        x = s8(x, s_box);
        let idx1 = ((r >> 8) & 0xFF) as usize;
        let idx2 = ((r >> 16) & 0xFF) as usize;
        s_box.swap(idx1, idx2);
    }

    x = x.rotate_right(!ki as u32);

    x
}
#[inline(never)]
pub fn ir8<K: Into<u128>>(
    mut y: u8,
    l: u8,
    i: usize,
    k: K,
    mut p_inv: Option<&mut [u8]>,
    mut s_inv: Option<&mut [u8]>,
) -> u8 {
    let k = k.into();
    let ki = k.rotate_right(i as u32) as u8;
    let r = mix64(
        k.rotate_right(l as u32) as u64 ^ (l as u64).rotate_left(17) ^ (i as u64).rotate_left(31),
    );

    y = y.rotate_left(!ki as u32);

    if let Some(s_inv_box) = &mut s_inv {
        y = s8(y, s_inv_box);

        let idx1 = ((r >> 8) & 0xFF) as u8;
        let idx2 = ((r >> 16) & 0xFF) as u8;
        let pos1 = s_inv_box.iter().position(|&v| v == idx1).unwrap();
        let pos2 = s_inv_box.iter().position(|&v| v == idx2).unwrap();
        s_inv_box.swap(pos1, pos2);
    }

    y = y.wrapping_mul(mix64_next(r) as u8 | 1);
    y = y.wrapping_mul(ki | 1);

    if let Some(p_inv_box) = &mut p_inv {
        y = p8(y, p_inv_box);

        let idx1 = (r & 0b111) as usize;
        let idx2 = ((r >> 3) & 0b111) as usize;
        p_inv_box.swap(idx1, idx2);
    }

    y ^ l ^ !(i as u8) ^ ki ^ r as u8
}

pub fn mse_no_s<K: Into<u128>, D: AsMut<[u8]>>(mut data: D, key: K, iv: u8) {
    let key = key.into();
    let data = data.as_mut();
    let mut prev = iv;
    let mut p = gen_p((key ^ key >> 64) as u64);

    for (i, pt) in data.iter_mut().enumerate() {
        let ct = r8(*pt, prev, i, key, Some(&mut p), None);
        prev = ct;
        *pt = ct;
    }
}
pub fn imse_no_s<K: Into<u128>, D: AsMut<[u8]>>(mut data: D, key: K, iv: u8) {
    let key = key.into();
    let data = data.as_mut();
    let mut prev = iv;

    let p_inv = gen_p((key ^ key >> 64) as u64);
    let mut p = inv_p(&p_inv).expect("P-Box generation failed");

    for (i, pt) in data.iter_mut().enumerate() {
        let ct = ir8(*pt, prev, i, key, Some(&mut p), None);
        prev = *pt;
        *pt = ct;
    }
}

pub fn mse_no_ps<K: Into<u128>, D: AsMut<[u8]>>(mut data: D, key: K, iv: u8) {
    let key = key.into();
    let data = data.as_mut();
    let mut prev = iv;

    for (i, pt) in data.iter_mut().enumerate() {
        let ct = r8(*pt, prev, i, key, None, None);
        prev = ct;
        *pt = ct;
    }
}
pub fn imse_no_ps<K: Into<u128>, D: AsMut<[u8]>>(mut data: D, key: K, iv: u8) {
    let key = key.into();
    let data = data.as_mut();
    let mut prev = iv;

    for (i, pt) in data.iter_mut().enumerate() {
        let ct = ir8(*pt, prev, i, key, None, None);
        prev = *pt;
        *pt = ct;
    }
}

pub fn mse<K: Into<u128>, D: AsMut<[u8]>>(mut data: D, key: K, iv: u8) {
    let key = key.into();
    let data = data.as_mut();
    let mut prev = iv;

    let seed_p = (key ^ (key >> 64)) as u64;
    let seed_s = mix64(seed_p);

    let mut p = gen_p(seed_p);
    let mut s = gen_s(seed_s);

    for (i, pt) in data.iter_mut().enumerate() {
        let ct = r8(*pt, prev, i, key, Some(&mut p), Some(&mut s));
        prev = ct;
        *pt = ct;
    }
}

pub fn imse<K: Into<u128>, D: AsMut<[u8]>>(mut data: D, key: K, iv: u8) {
    let key = key.into();
    let data = data.as_mut();
    let mut prev = iv;

    let seed_p = (key ^ (key >> 64)) as u64;
    let seed_s = mix64(seed_p);

    let p_box = gen_p(seed_p);
    let s_box = gen_s(seed_s);

    let mut p_inv = inv_p(&p_box).expect("P-Box generation failed");
    let mut s_inv = inv_s(&s_box).expect("S-Box generation failed");

    for (i, pt) in data.iter_mut().enumerate() {
        let ct = ir8(*pt, prev, i, key, Some(&mut p_inv), Some(&mut s_inv));
        prev = *pt;
        *pt = ct;
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::rand::{fill_bytes, next};

    #[test]
    fn test_sbox_base_inverse() {
        let inv = inv_s(&SBOX_BASE[0]).expect("SBOX_BASE[0] 应该具备逆矩阵");
        assert_eq!(inv, SBOX_BASE[1], "硬编码的逆矩阵有误");

        // 验证双射性
        for i in 0..=255u8 {
            let encrypted = s8(i, &SBOX_BASE[0]);
            let decrypted = s8(encrypted, &SBOX_BASE[1]);
            assert_eq!(i, decrypted);
        }
    }

    #[test]
    fn test_inv_mul8() {
        // 验证所有奇数在 mod 256 下的乘法逆元是否正确
        for m in (1..=255).step_by(2) {
            let inv = inv_mul8(m);
            assert_eq!(m.wrapping_mul(inv), 1, "{} 的逆元计算错误: {}", m, inv);
        }
    }

    #[test]
    fn test_shuffle_with() {
        let original = [0, 1, 2, 3, 4, 5, 6, 7];
        let shuffled = shuffle_with(original, 12345);

        assert_eq!(original.len(), shuffled.len());
    }

    #[test]
    fn test_gen_and_inv_boxes() {
        let seed = 998244353;

        // 测试 P-Box
        let p_box = gen_p(seed);
        let p_inv = inv_p(&p_box).expect("生成的 P-box 不是合法的置换");
        for x in 0..=255u8 {
            let p = p8(x, &p_box);
            let original = p8(p, &p_inv);
            assert_eq!(x, original, "P-Box 还原失败");
        }

        // 测试 S-Box
        let s_box = gen_s(seed);
        let s_inv = inv_s(&s_box).expect("生成的 S-box 不是合法的置换");
        for x in 0..=255u8 {
            let s = s8(x, &s_box);
            let original = s8(s, &s_inv);
            assert_eq!(x, original, "S-Box 还原失败");
        }
    }

    #[test]
    fn test_light_hasher() {
        let hash1 = derive("Hello, World!");
        let hash2 = derive("Hello, World!");
        let hash3 = derive("Hello, Rust!");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_r8_ir8_without_boxes() {
        let pt = 0x42; // 明文
        let l = 0xAA; // 模拟上一个密文
        let i = 5; // 轮数
        let k = 0x1234567890ABCDEF_u128; // 密钥

        let ct = r8(pt, l, i, k, None, None);
        let decrypted = ir8(ct, l, i, k, None, None);

        assert_eq!(pt, decrypted, "无Box情况下的核心加解密失败");
    }

    #[test]
    fn test_r8_ir8_with_dynamic_boxes() {
        let pt = 0x7F;
        let l = 0x11;
        let i = 10;
        let k = 0x1234567890ABCDEF_u128;

        let mut p_box = gen_p(111);
        let mut s_box = gen_s(222);

        let mut p_inv = inv_p(&p_box).unwrap();
        let mut s_inv = inv_s(&s_box).unwrap();

        let ct = r8(pt, l, i, k, Some(&mut p_box), Some(&mut s_box));

        let decrypted = ir8(ct, l, i, k, Some(&mut p_inv), Some(&mut s_inv));

        assert_eq!(pt, decrypted, "动态Box情况下的核心解密失败");

        let expected_p_inv = inv_p(&p_box).unwrap();
        let expected_s_inv = inv_s(&s_box).unwrap();

        assert_eq!(
            p_inv, expected_p_inv,
            "加密/解密后 P-Box 的内部状态发生错乱不同步"
        );
        assert_eq!(
            s_inv, expected_s_inv,
            "加密/解密后 S-Box 的内部状态发生错乱不同步"
        );
    }

    // fill_bytes 9.29µs
    // enc 346.743µs
    // dec 1.0083ms
    #[cfg(feature = "std")]
    #[test]
    fn test_end_to_end_buffer_encryption() {
        use std::collections::HashSet;
        use std::time::Instant;
        let mut plaintext = vec![0; 1024];
        let _start = Instant::now();
        fill_bytes(&mut plaintext);
        println!("fill_bytes {:?}", _start.elapsed());

        let key = 0x9988776655443322;

        let mut p_box = gen_p(key);
        let mut s_box = gen_s(key);

        let mut p_inv = inv_p(&p_box).unwrap();
        let mut s_inv = inv_s(&s_box).unwrap();

        let mut ciphertext = Vec::with_capacity(plaintext.len());
        let mut prev_c = 0u8; // 初始 IV

        let _start = Instant::now();
        for (i, &pt) in plaintext.iter().enumerate() {
            let ct = r8(pt, prev_c, i, key, Some(&mut p_box), Some(&mut s_box));
            ciphertext.push(ct);
            prev_c = ct;
        }
        println!("enc {:?}", _start.elapsed());

        let mut decrypted = Vec::with_capacity(ciphertext.len());
        prev_c = 0u8;

        let _start = Instant::now();
        for (i, &ct) in ciphertext.iter().enumerate() {
            let pt = ir8(ct, prev_c, i, key, Some(&mut p_inv), Some(&mut s_inv));
            decrypted.push(pt);
            prev_c = ct;
        }
        println!("dec {:?}", _start.elapsed());

        println!("ciphertext: {:?}", ciphertext);

        assert_eq!(
            plaintext, decrypted,
            "长文本流连续加解密失败，状态未能正确传递"
        );

        mse_no_ps(&mut decrypted, key, 0);
        imse_no_ps(&mut decrypted, key, 0);
        assert_eq!(plaintext, decrypted, "mse_no_sp,imse_no_sp失败");

        mse_no_s(&mut decrypted, key, 0);
        imse_no_s(&mut decrypted, key, 0);
        assert_eq!(plaintext, decrypted, "mse_no_s,imse_no_s失败");

        mse(&mut decrypted, key, 0);
        imse(&mut decrypted, key, 0);
        assert_eq!(plaintext, decrypted, "mse_no_s,imse_no_s失败");
    }
    #[cfg(feature = "std")]
    #[test]
    fn test_mix() {
        use std::collections::HashSet;
        use std::time::Instant;
        let mut s = 0;
        for _ in 0..1000 {
            let t = Instant::now();
            s = mix64_next(s);
            println!("took: {:<10.2?} {s}", t.elapsed());
        }
    }
}
