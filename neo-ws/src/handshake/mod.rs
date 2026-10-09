mod key;
mod writer;

use crate::handshake::writer::{BufTooSmall, WsBufWriter};

pub use key::HandshakeKey;

pub struct Handshake {
    key: HandshakeKey,
}

impl Handshake {
    pub fn new(key: HandshakeKey) -> Self {
        Self { key }
    }

    pub fn request(&self, buf: &mut [u8], host: &str, path: &str) -> Result<usize, BufTooSmall> {
        let mut writer = WsBufWriter::new(buf);

        let key = self.key.encode_base64();

        writer.write_line3("GET ", path, " HTTP/1.1")?;
        writer.write_line2("Host: ", host)?;
        writer.write_line("Upgrade: websocket")?;
        writer.write_line("Connection: Upgrade")?;
        writer.write_line2sb("Sec-WebSocket-Key: ", &key)?;
        writer.write_line("Sec-WebSocket-Version: 13")?;
        writer.write_crlf()?;

        let written = writer.len();

        Ok(written)
    }

    pub fn response(self, input: &[u8]) -> Result<HandshakeStep, Box<dyn std::error::Error>> {
        todo!()
    }
}

pub enum HandshakeStep {
    Incomplete(Handshake),
    Done { conn: (), consumed: usize },
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn handshake_req() {
        const HOST: &str = "test.host";
        const PATH: &str = "/path";

        let key = HandshakeKey::new_random().unwrap();
        let key_b64 = key.encode_base64();

        let handshake = Handshake::new(key);

        let mut buf = vec![0; 2048];
        let n = handshake.request(&mut buf, HOST, PATH).unwrap();

        let request = str::from_utf8(&buf[..n]).unwrap();

        let expected = format!(
            "GET {PATH} HTTP/1.1\r\n\
             Host: {HOST}\r\n\
             Upgrade: websocket\r\n\
             Connection: Upgrade\r\n\
             Sec-WebSocket-Key: {}\r\n\
             Sec-WebSocket-Version: 13\r\n\r\n",
            std::str::from_utf8(&key_b64).unwrap(),
        );

        assert_eq!(request, expected);
    }

    #[test]
    fn test_small_buf() {
        let key = HandshakeKey::new_random().unwrap();
        let handshake = Handshake::new(key);

        let mut buf = vec![0; 4];
        let resp = handshake.request(&mut buf, "host", "/path");

        assert!(resp.is_err());
    }
}
