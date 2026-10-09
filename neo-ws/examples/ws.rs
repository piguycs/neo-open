use std::{
    io::{Read, Write},
    net::{Ipv4Addr, TcpStream},
};

use neo_ws::handshake::{Handshake, HandshakeKey, HandshakeStep};

// Run `websocat -s 8080` or something similar for this example!
fn main() {
    let key = HandshakeKey::new_random().expect("could not generate random bytes");
    let handshake = Handshake::new(key);

    let mut buf = vec![0; 2048];
    let n = handshake.request(&mut buf, "localhost", "/").unwrap();
    let write_buf = &buf[..n];

    let mut tcp = TcpStream::connect((Ipv4Addr::UNSPECIFIED, 8080)).unwrap();

    tcp.write_all(write_buf).unwrap();

    let n = tcp.read(&mut buf).unwrap();

    let step = handshake.response(&buf[..n]).unwrap();
    let (_conn, consumed) = match step {
        HandshakeStep::Incomplete(_) => panic!("unable to complete handshake"),
        HandshakeStep::Done { conn, consumed } => (conn, consumed),
    };

    println!("handshake was success! (consumed={consumed},n={n})")
}
