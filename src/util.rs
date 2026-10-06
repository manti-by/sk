//! Small filesystem helpers and content hashing.

use std::io::Write;
use std::path::Path;

use crate::error::{Error, Result};

/// Round constants of SHA-256: the first 32 bits of the fractional parts of the
/// cube roots of the first sixty-four primes.
const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// SHA-256 of `bytes`, lowercase hex.
///
/// Spelled out here rather than pulled in as a dependency: it is the only hash
/// `sk` needs, and a digest the user can reproduce with `shasum -a 256` is worth
/// more in `sk.lock` than the lines a crate would have saved.
pub fn sha256_hex(bytes: &[u8]) -> String {
    // Pad to a multiple of 64 bytes: a 0x80 byte, zeroes, then the message
    // length in bits as a big-endian u64.
    let mut message = bytes.to_vec();
    let bit_len = (bytes.len() as u64).wrapping_mul(8);
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());

    // Initial state: first 32 bits of the fractional parts of the square roots
    // of the first eight primes.
    let mut state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    for block in message.as_chunks::<64>().0 {
        let mut w = [0u32; 64];
        for (word, bytes) in w.iter_mut().zip(block.as_chunks::<4>().0) {
            *word = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(SHA256_K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        for (slot, value) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *slot = slot.wrapping_add(value);
        }
    }

    state.iter().flat_map(|word| word.to_be_bytes()).fold(
        String::with_capacity(64),
        |mut hex, byte| {
            use std::fmt::Write as _;
            let _ = write!(hex, "{byte:02x}");
            hex
        },
    )
}

/// Writes a file via a temporary sibling, so readers never see a partial file
/// and a failed write leaves the previous content in place.
pub fn write_atomic(path: &Path, contents: &[u8]) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent).map_err(|source| {
        Error::with_source(format!("could not create {}", parent.display()), source)
    })?;

    let mut file_name = path
        .file_name()
        .ok_or_else(|| Error::new(format!("{} has no file name", path.display())))?
        .to_os_string();
    file_name.push(".tmp");
    let temp = parent.join(file_name);

    let mut file = std::fs::File::create(&temp).map_err(|source| {
        Error::with_source(format!("could not create {}", temp.display()), source)
    })?;
    let write_result = file.write_all(contents).and_then(|()| file.sync_all());
    if let Err(source) = write_result {
        std::fs::remove_file(&temp).ok();
        return Err(Error::with_source(
            format!("could not write {}", temp.display()),
            source,
        ));
    }
    drop(file);

    std::fs::rename(&temp, path).map_err(|source| {
        std::fs::remove_file(&temp).ok();
        Error::with_source(format!("could not update {}", path.display()), source)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_and_replaces_atomically() {
        let dir = std::env::temp_dir().join(format!("sk-util-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("nested").join("file.txt");

        write_atomic(&path, b"first").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"first");

        write_atomic(&path, b"second").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"second");

        assert!(!dir.join("nested").join("file.txt.tmp").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn hashes_match_the_published_sha256_vectors() {
        // Vectors from FIPS 180-4 and NIST's SHA-256 examples.
        for (input, expected) in [
            (
                b"" as &[u8],
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            (
                b"abc",
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            ),
            (
                b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
                "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
            ),
        ] {
            assert_eq!(sha256_hex(input), expected, "digest of {input:?}");
        }
    }

    #[test]
    fn hashing_covers_the_block_and_length_padding_edges() {
        // Digests of runs of `a`, so a padding bug on a multi-block message shows
        // up: 55 bytes still fit one block, 56 needs another, and 64 fills one
        // exactly. 1000 bytes runs well past two blocks.
        for (len, expected) in [
            (
                55,
                "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318",
            ),
            (
                56,
                "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a",
            ),
            (
                64,
                "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb",
            ),
            (
                65,
                "635361c48bb9eab14198e76ea8ab7f1a41685d6ad62aa9146d301d4f17eb0ae0",
            ),
            (
                1000,
                "41edece42d63e8d9bf515a9ba6932e1c20cbc9f5a5d134645adb5db1b9737ea3",
            ),
        ] {
            assert_eq!(
                sha256_hex(&vec![b'a'; len]),
                expected,
                "digest of {len} bytes"
            );
        }
        assert_ne!(sha256_hex(b"a"), sha256_hex(b"b"));
    }
}
