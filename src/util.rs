pub fn u64_to_le_bytes(n: u64, nbytes: usize) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(nbytes);
    for i in range(0, nbytes) {
        bytes.push((n >> (i * 8)) as u8);
    }
    bytes
}

pub fn bytes_to_le_u64(bytes: &[u8]) -> u64 {
    let mut n = 0;
    let mut shift = 0;
    for b in bytes.iter() {
        n += (*b as u64) << shift;
        shift += 8;
    }
    n
}

pub fn u64_to_be_bytes(n: u64, nbytes: usize) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(nbytes);
    for i in range(1, nbytes + 1) {
        bytes.push((n >> ((nbytes - i) * 8)) as u8);
    }
    bytes
}

pub fn bytes_to_be_u64(bytes: &[u8]) -> u64 {
    let mut n = 0;
    for b in bytes.iter() {
        n = (n << 8) | *b as u64;
    }
    n
}
