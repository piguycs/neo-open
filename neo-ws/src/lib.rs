//! A Websocket parser.
//!
//! The core of this library is split into two modules
//! - [handshake]: Perform a HTTP/1.1 handshake to start a Websocket connection
//! - [connection]: The state machine which handles the Websocket protocol's stream

pub mod connection;
pub mod handshake;
