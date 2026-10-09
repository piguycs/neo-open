mod key;
mod writer;

use crate::{
    connection::Connection,
    handshake::writer::{BufTooSmall, WsBufWriter},
};

pub use key::HandshakeKey;

pub const CRLF: &[u8] = b"\r\n";

pub type HandshakeResult = Result<HandshakeStep, HandshakeError>;

#[derive(Debug)]
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

    /// Handle the response, and upgrade the handshake struct to a websocket connection when
    /// everything is a success.
    ///
    /// The client is expected to use the return value to first verify if the handshake was
    /// successfully completed. If it was incomplete, then the response struct must be called again
    /// with more data passed to the input. There is no internal tracking of previously parsed
    /// bytes, so the whole buffer must be passed again.
    ///
    /// If the upgrade was successful, a new struct will be returned, one to represent the active
    /// websocket connection.
    pub fn response(self, input: &[u8]) -> HandshakeResult {
        const HEADER_SPLIT: &[u8] = b":";
        const HTTP_1_1: &[u8] = b"HTTP/1.1";
        const STATUS_SWITCHING: &[u8] = b"101";

        let input_len = input.len();

        let Some((http_start, mut input)) = bytes_split_once(input, CRLF) else {
            return Ok(HandshakeStep::Incomplete(self));
        };

        let mut iter = http_start.splitn(3, |&b| b == b' ');

        let http_version_valid = match iter.next() {
            Some(http_ver) => http_ver == HTTP_1_1,
            None => return Ok(HandshakeStep::Incomplete(self)),
        };

        if !http_version_valid {
            return Err(HandshakeError::HttpVersionInvalid);
        }

        let status_valid = match iter.next() {
            Some(status) => status == STATUS_SWITCHING,
            None => return Ok(HandshakeStep::Incomplete(self)),
        };

        if !status_valid {
            return Err(HandshakeError::HttpStatusNotSwitching);
        }

        let mut upgrade_valid = false;
        let mut connection_valid = false;
        let mut accept = None;

        let mut eof = false;

        while let Some((header_line, rest)) = bytes_split_once(input, CRLF) {
            input = rest;

            if header_line.is_empty() {
                eof = true;
                break;
            }

            let Some((key, value)) = bytes_split_once(header_line, HEADER_SPLIT) else {
                // invalid_header_cb(header_line);
                continue;
            };

            // there can be any number of whitespaces around these values
            let key = key.trim_ascii();
            let value = value.trim_ascii();

            match key {
                key if key.eq_ignore_ascii_case(b"Upgrade") => {
                    upgrade_valid |= value.eq_ignore_ascii_case(b"websocket");
                }

                key if key.eq_ignore_ascii_case(b"Connection") => {
                    connection_valid |= value
                        .split(|&b| b == b',')
                        .any(|token| token.trim_ascii().eq_ignore_ascii_case(b"Upgrade"));
                }

                key if key.eq_ignore_ascii_case(b"Sec-WebSocket-Accept") => accept = Some(value),

                _ => continue,
            }
        }

        if !eof {
            return Ok(HandshakeStep::Incomplete(self));
        }

        if !upgrade_valid || !connection_valid {
            return Err(HandshakeError::HeadersInvalid {
                upgrade_valid,
                connection_valid,
            });
        }

        // TODO: hash validation
        let _ = accept;
        let hash_valid = true;

        if !hash_valid {
            return Err(HandshakeError::HashInvalid);
        }

        Ok(HandshakeStep::Done {
            conn: Connection::init(),
            consumed: input_len - input.len(),
        })
    }
}

#[derive(Debug)]
pub enum HandshakeStep {
    Incomplete(Handshake),
    Done { conn: Connection, consumed: usize },
}

#[derive(Debug, PartialEq, Eq)]
pub enum HandshakeError {
    NotSwitchingProtocols,
    HashInvalid,
    HeadersInvalid {
        upgrade_valid: bool,
        connection_valid: bool,
    },
    HttpVersionInvalid,
    HttpStatusNotSwitching,
}

impl std::fmt::Display for HandshakeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandshakeError::NotSwitchingProtocols => {
                write!(f, "The endpoint did not switch to the websocket protocol")
            }
            HandshakeError::HashInvalid => {
                write!(f, "The hash sent by the server is invalid")
            }
            HandshakeError::HeadersInvalid {
                upgrade_valid,
                connection_valid,
            } => write!(
                f,
                "Some or all of the headers are invalid. upgrade_valid={},connection_valid={}",
                upgrade_valid, connection_valid,
            ),
            HandshakeError::HttpVersionInvalid => write!(f, "Response HTTP version is invalid"),
            HandshakeError::HttpStatusNotSwitching => write!(f, "Response HTTP status is not 101"),
        }
    }
}

impl std::error::Error for HandshakeError {}

fn bytes_split_once<'a>(input: &'a [u8], pattern: &[u8]) -> Option<(&'a [u8], &'a [u8])> {
    let pos = input.windows(pattern.len()).position(|w| w == pattern)?;

    let first_end = pos;
    let second_start = first_end + pattern.len();

    Some((&input[..first_end], &input[second_start..]))
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

    fn get_handshaked() -> Handshake {
        let key = HandshakeKey::new_random().unwrap();
        Handshake::new(key)
    }

    #[test]
    fn no_splits_in_response() {
        let hs = get_handshaked();
        let step = hs.response(b"HTTP/1.1 200 Halloooo").unwrap();

        assert!(matches!(step, HandshakeStep::Incomplete(_)));
    }

    #[test]
    fn incorrect_http() {
        let hs = get_handshaked();
        let step = hs.response(b"HTTP/2.1 101 Switching\r\n\r\n");

        assert_eq!(step.unwrap_err(), HandshakeError::HttpVersionInvalid);
    }

    #[test]
    fn incorrect_status() {
        let hs = get_handshaked();
        let step = hs.response(b"HTTP/1.1 500 Oops\r\n\r\n");

        assert_eq!(step.unwrap_err(), HandshakeError::HttpStatusNotSwitching);
    }

    #[test]
    fn no_headers() {
        let hs = get_handshaked();
        let step = hs.response(b"HTTP/1.1 101 Switching\r\n\r\n");

        assert_eq!(
            step.unwrap_err(),
            HandshakeError::HeadersInvalid {
                upgrade_valid: false,
                connection_valid: false,
            }
        );
    }

    #[test]
    fn valid_resp() {
        let hs = get_handshaked();
        let req = b"HTTP/1.1 101 Switching\r\nUpgrade: WebSocket\r\nConnection: upgrade\r\n\r\n";
        let step = hs.response(req);

        assert!(matches!(
            step.unwrap(),
            HandshakeStep::Done {
                consumed,
                ..
            } if consumed == req.len()
        ));
    }

    #[test]
    fn skip_invalid_headers() {
        let hs = get_handshaked();
        let req = b"HTTP/1.1 101 Switching\r\nINVALIDHEADER\r\nUpgrade: WebSocket\r\nConnection: upgrade\r\n\r\n";
        let step = hs.response(req);

        assert!(matches!(
            step.unwrap(),
            HandshakeStep::Done {
                consumed,
                ..
            } if consumed == req.len()
        ));
    }

    // TODO: some test about sec-websocket-key would be good, once I implement that logic (if I do it)
}
