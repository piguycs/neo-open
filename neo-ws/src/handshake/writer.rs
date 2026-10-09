use crate::handshake::CRLF;

/// This writer is made specifically for writing specific HTTP headers for the Websocket handshake.
/// The weird functions are written such that the caller code can stay "clean" and "reviewable".
pub struct WsBufWriter<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> WsBufWriter<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub fn len(&self) -> usize {
        self.pos
    }

    pub fn write_line(&mut self, s: &str) -> Result<(), BufTooSmall> {
        self.write_str(s)?;
        self.write_crlf()
    }

    pub fn write_line2(&mut self, s1: &str, s2: &str) -> Result<(), BufTooSmall> {
        self.write_str(s1)?;
        self.write_str(s2)?;
        self.write_crlf()
    }

    pub fn write_line2sb(&mut self, s1: &str, b1: &[u8]) -> Result<(), BufTooSmall> {
        self.write_str(s1)?;
        self.write_bytes(b1)?;
        self.write_crlf()
    }

    pub fn write_line3(&mut self, s1: &str, s2: &str, s3: &str) -> Result<(), BufTooSmall> {
        self.write_str(s1)?;
        self.write_str(s2)?;
        self.write_str(s3)?;
        self.write_crlf()
    }

    pub fn write_crlf(&mut self) -> Result<(), BufTooSmall> {
        self.write_bytes(CRLF)
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), BufTooSmall> {
        let start = self.pos;
        let end = start + bytes.len();

        let dst = self.buf.get_mut(start..end).ok_or(BufTooSmall)?;
        dst.copy_from_slice(bytes);
        self.pos += bytes.len();

        Ok(())
    }

    fn write_str(&mut self, s: &str) -> Result<(), BufTooSmall> {
        self.write_bytes(s.as_bytes())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BufTooSmall;

impl std::fmt::Display for BufTooSmall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Buffer is too small")
    }
}

impl std::error::Error for BufTooSmall {}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn write() {
        let mut buf = [0; 256];
        let mut w = WsBufWriter::new(&mut buf);

        w.write_line("GET / HTTP/1.1").unwrap();
        w.write_line2("Host: ", "test.host").unwrap();
        w.write_line2sb("Key: ", b"abc123").unwrap();
        w.write_line3("Version: ", "1", "3").unwrap();
        w.write_crlf().unwrap();

        let n = w.len();

        assert_eq!(
            &buf[..n],
            b"GET / HTTP/1.1\r\n\
              Host: test.host\r\n\
              Key: abc123\r\n\
              Version: 13\r\n\r\n"
        );
    }
}
