const MH_C1: u64 = 0x87c37b91114253d5;
const MH_C2: u64 = 0x4cf5ad432745937f;
const MEMHASH_SEED: u64 = 0x71e729fd56419c81;

fn fmix64(mut k: u64) -> u64 {
    k ^= k >> 33;
    k = k.wrapping_mul(0xff51afd7ed558ccd);
    k ^= k >> 33;
    k = k.wrapping_mul(0xc4ceb9fe1a85ec53);
    k ^= k >> 33;
    k
}

fn murmur_x64_128_h2(data: &[u8], seed: u32) -> u64 {
    let mut h1 = seed as u64;
    let mut h2 = seed as u64;
    let nblocks = data.len() / 16;
    for i in 0..nblocks {
        let mut k1 = u64::from_le_bytes(data[i * 16..i * 16 + 8].try_into().expect("8 bytes"));
        let mut k2 = u64::from_le_bytes(data[i * 16 + 8..i * 16 + 16].try_into().expect("8 bytes"));
        k1 = k1.wrapping_mul(MH_C1);
        k1 = k1.rotate_left(31);
        k1 = k1.wrapping_mul(MH_C2);
        h1 ^= k1;
        h1 = h1.rotate_left(27);
        h1 = h1.wrapping_add(h2);
        h1 = h1.wrapping_mul(5).wrapping_add(0x52dce729);
        k2 = k2.wrapping_mul(MH_C2);
        k2 = k2.rotate_left(33);
        k2 = k2.wrapping_mul(MH_C1);
        h2 ^= k2;
        h2 = h2.rotate_left(31);
        h2 = h2.wrapping_add(h1);
        h2 = h2.wrapping_mul(5).wrapping_add(0x38495ab5);
    }
    let tail = &data[nblocks * 16..];
    let mut k1 = 0u64;
    let mut k2 = 0u64;
    if tail.len() >= 15 {
        k2 ^= (tail[14] as u64) << 48;
    }
    if tail.len() >= 14 {
        k2 ^= (tail[13] as u64) << 40;
    }
    if tail.len() >= 13 {
        k2 ^= (tail[12] as u64) << 32;
    }
    if tail.len() >= 12 {
        k2 ^= (tail[11] as u64) << 24;
    }
    if tail.len() >= 11 {
        k2 ^= (tail[10] as u64) << 16;
    }
    if tail.len() >= 10 {
        k2 ^= (tail[9] as u64) << 8;
    }
    if tail.len() >= 9 {
        k2 ^= tail[8] as u64;
        k2 = k2.wrapping_mul(MH_C2);
        k2 = k2.rotate_left(33);
        k2 = k2.wrapping_mul(MH_C1);
        h2 ^= k2;
    }
    if tail.len() >= 8 {
        k1 ^= (tail[7] as u64) << 56;
    }
    if tail.len() >= 7 {
        k1 ^= (tail[6] as u64) << 48;
    }
    if tail.len() >= 6 {
        k1 ^= (tail[5] as u64) << 40;
    }
    if tail.len() >= 5 {
        k1 ^= (tail[4] as u64) << 32;
    }
    if tail.len() >= 4 {
        k1 ^= (tail[3] as u64) << 24;
    }
    if tail.len() >= 3 {
        k1 ^= (tail[2] as u64) << 16;
    }
    if tail.len() >= 2 {
        k1 ^= (tail[1] as u64) << 8;
    }
    if !tail.is_empty() {
        k1 ^= tail[0] as u64;
        k1 = k1.wrapping_mul(MH_C1);
        k1 = k1.rotate_left(31);
        k1 = k1.wrapping_mul(MH_C2);
        h1 ^= k1;
    }
    let len = data.len() as u64;
    h1 ^= len;
    h2 ^= len;
    h1 = h1.wrapping_add(h2);
    h2 = h2.wrapping_add(h1);
    h1 = fmix64(h1);
    h2 = fmix64(h2);
    h1 = h1.wrapping_add(h2);
    h2 = h2.wrapping_add(h1);
    h2
}

pub fn julia_string_hash(s: &str) -> u64 {
    let h = MEMHASH_SEED;
    murmur_x64_128_h2(s.as_bytes(), (h & 0xffff_ffff) as u32).wrapping_add(h)
}
