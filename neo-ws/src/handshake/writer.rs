pub const CRLF: &[u8] = b"\r\n";

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
