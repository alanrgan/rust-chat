extern crate mio;
extern crate http_muncher;
extern crate sha1;
extern crate rustc_serialize;
extern crate byteorder;

use mio::*;
use mio::deprecated::EventLoop;
use mio::tcp::*;
use std::net::SocketAddr;
use std::collections::HashMap;

mod client;
mod server;
mod frame;
pub use client::WebSocketClient;
use server::WebSocketServer;

fn main() {
    let mut event_loop = EventLoop::new().unwrap();

    let address = "0.0.0.0:10000".parse::<SocketAddr>().unwrap();
    let server_socket = TcpListener::bind(&address).unwrap();

    let mut server = WebSocketServer {
    	token_counter: 1,
    	clients: HashMap::new(),
    	socket: server_socket
    };

    event_loop.register(&server.socket,
    					Token(0),
    					Ready::readable(),
    					PollOpt::edge()).unwrap();

    event_loop.run(&mut server).unwrap();
}
