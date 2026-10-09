/// The handshake key is a 16 byte randomly generated base64 encoded string, as specified by the
/// websocket protocol RFC.
pub struct HandshakeKey([u8; 16]);

impl HandshakeKey {
    pub fn new_random() -> Option<Self> {
        let mut rand_key = [0; 16];
        getrandom::fill(&mut rand_key).ok()?;

        Some(Self(rand_key))
    }

    /// Encode the key as base64.
    ///
    /// This function uses a hand-rolled implementation of base64, as it is quite reasonable for a
    /// fixed-sized input/output as required for this function.
    pub fn encode_base64(&self) -> [u8; 24] {
        const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

        let (chunks, remainder) = self.0.as_chunks::<3>();
        let mut out = [b'='; 24];

        for (src, dst) in chunks.iter().zip(out.as_chunks_mut::<4>().0) {
            dst[0] = B64[(src[0] >> 2) as usize];
            dst[1] = B64[((src[0] & 0x03) << 4 | src[1] >> 4) as usize];
            dst[2] = B64[((src[1] & 0x0f) << 2 | src[2] >> 6) as usize];
            dst[3] = B64[(src[2] & 0x3f) as usize];
        }

        out[20] = B64[(remainder[0] >> 2) as usize];
        out[21] = B64[((remainder[0] & 0x03) << 4) as usize];

        out
    }
}

#[cfg(test)]
mod test {

    use super::*;

    /// Compares my hand-rolled base64 implementation with a third-party implementation from the
    /// [data_encoding] crate. This is a soundness test, and the third-party crate will only be used
    /// as a dev-dependency.
    #[test]
    fn compare_base64() {
        for _ in 0..10_000 {
            let key = HandshakeKey::new_random().unwrap();

            let expected = data_encoding::BASE64.encode(&key.0);
            let actual = key.encode_base64();

            assert_eq!(actual.as_slice(), expected.as_bytes());
        }
    }
}
