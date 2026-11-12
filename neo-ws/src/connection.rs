use std::array;

pub type ConnResult<T> = Result<(Connection, usize, Option<T>), ConnError>;

#[derive(Debug)]
pub struct Connection {}

impl Connection {
    pub fn init() -> Self {
        Self {}
    }

    /// In both those cases, it is not the job of this module to handle "whatever happens next".
    ///
    /// API for consuming bytes from the incoming stream. In case of failures, the connection
    /// becomes unusable.
    ///
    /// Must be provided with the last event it parsed. If it is the first call, provide [None].
    pub fn consume(self, last_event: Option<WsEvent>, data: &[u8]) -> ConnResult<()> {
        let last_event = last_event.unwrap_or(WsEvent::FrameEnd);

        let event = match last_event {
            WsEvent::FrameStart(_) => todo!(),

            WsEvent::FrameEnd => {
                Self::parse_frame_start(data).map(|(i, e)| (i, WsEvent::FrameStart(e)))
            }
        };

        // check what the last event was or fuck it we ball?

        if data.len() < 2 {
            return Ok((self, 0, None));
        }

        // these are 5 bytes, perhaps better to just store two bytes, and then provide a view into
        // them on demand?

        let _fin = data[0] & 0b1000_0000 != 0;
        let _rsv = data[0] & 0b0111_0000;
        let _opcode = data[0] & 0b0000_1111;

        let _masked = data[1] & 0b1000_0000 != 0;
        let _len_indicator = data[1] & 0b0111_1111;

        todo!()
    }

    fn parse_frame_start(data: &[u8]) -> Option<(usize, FrameStart)> {
        let frame_def = FrameDef::new(data)?;

        let fin = data[0] & 0b1000_0000 != 0;
        // as per spec, we should verify this is zero, as we do not support any extensions which
        // make use of this reserved space. But on the other hand, idrc
        let _rsv = data[0] & 0b0111_0000;
        let opcode = data[0] & 0b0000_1111;

        let masked = data[1] & 0b1000_0000 != 0;

        let len: u64 = match frame_def.len_indicator() {
            LenIndicator::SevenBit(len) => len as u64,
            LenIndicator::TwoByte => {
                let bytes: [u8; 2] = get_array(&data[3..])?;
                u16::from_be_bytes(bytes) as u64
            }
            LenIndicator::EightByte => {
                let bytes: [u8; 8] = get_array(&data[3..])?;
                u64::from_be_bytes(bytes)
            }
        };

        let event = FrameStart {
            fin,
            len,
            opcode,
            masked,
        };

        // we can construct the "framedef" struct here, which gives us a view into the first couple
        // bytes

        todo!()
    }
}

pub struct FrameDef {
    inner: [u8; 2],
}

impl FrameDef {
    pub fn new(data: &[u8]) -> Option<Self> {
        let (inner, _) = data.split_first_chunk::<2>()?;
        Some(Self { inner: *inner })
    }

    pub fn len_indicator(&self) -> LenIndicator {
        const LEN_TWO_BYTES: u8 = 126;
        const LEN_EIGHT_BYTES: u8 = 127;

        let indicator = self.inner[1] & 0b0111_1111;

        match indicator {
            LEN_TWO_BYTES => LenIndicator::TwoByte,
            LEN_EIGHT_BYTES => LenIndicator::EightByte,
            len => LenIndicator::SevenBit(len),
        }
    }
}

pub enum LenIndicator {
    SevenBit(u8),
    TwoByte,
    EightByte,
}

/// Websocket connection events. These broadcast all events which conspire on the websocket
/// connection. The caller must make sense of these events
pub enum WsEvent {
    FrameStart(FrameStart),
    FrameEnd,
}

pub struct FrameStart {
    fin: bool,
    len: u64,
    opcode: u8,
    masked: bool,
}

pub enum ConnError {}

/// Get a fixed sized array if the provided slice has the required number of elements
fn get_array<const N: usize, T: Copy>(data: &[T]) -> Option<[T; N]> {
    let slice = data.get(..N)?;
    Some(array::from_fn(|i| slice[i]))
}
