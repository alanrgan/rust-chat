pub use self::client::WebSocketClient;

mod client {
	extern crate mio;
	extern crate http_muncher;
	extern crate sha1;
	extern crate rustc_serialize;

	use rustc_serialize::base64::{ToBase64, STANDARD};
	use http_muncher::{Parser, ParserHandler};

	use mio::*;
	use mio::tcp::*;
	use std::collections::HashMap;
	use std::io::{Read, Write};
	use std::cell::RefCell;
	use std::rc::Rc;
	use std::fmt;
	use std;

	use frame::WebSocketFrame;
	use frame::OpCode;

	fn gen_key(key: &String) -> String {
		let mut m = sha1::Sha1::new();
		let mut buf = [0u8; 20];

		m.update(key.as_bytes());
		m.update("258EAFA5-E914-47DA-95CA-C5AB0DC85B11".as_bytes());

		m.output(&mut buf);
		return buf.to_base64(STANDARD);
	}

	struct HttpParser {
		current_key: Option<String>,
		headers: Rc<RefCell<HashMap<String, String>>>
	}

	impl ParserHandler for HttpParser {
		fn on_header_field(&mut self, s: &[u8]) -> bool {
			self.current_key = Some(std::str::from_utf8(s).unwrap().to_string());
			true
		}

		fn on_header_value(&mut self, s: &[u8]) -> bool {
			self.headers.borrow_mut()
				.insert(self.current_key.clone().unwrap(),
						std::str::from_utf8(s).unwrap().to_string());
				true
		}

		fn on_headers_complete(&mut self) -> bool {
			false
		}
	}

	enum ClientState {
		AwaitingHandshake(RefCell<Parser<HttpParser>>),
		HandshakeResponse,
		Connected
	}

	pub struct WebSocketClient {
		pub socket: TcpStream,
		headers: Rc<RefCell<HashMap<String, String>>>,
		pub interest: Ready,
		state: ClientState,
		outgoing: Vec<WebSocketFrame>
	}

	impl WebSocketClient {
		pub fn read(&mut self) -> Option<String> {
			match self.state {
				ClientState::AwaitingHandshake(_) => self.read_handshake(),
				ClientState::Connected => self.read_frame(),
				_ => None
			}
		}

		fn read_frame(&mut self) -> Option<String> {
			let frame = WebSocketFrame::read(&mut self.socket);
			match frame {
				Ok(frame) => {
					match frame.get_opcode() {
						OpCode::TextFrame => {
							println!("{}", std::str::from_utf8(&frame.payload).unwrap());
							let reply_frame = WebSocketFrame::from("Hi there!");
							self.outgoing.push(reply_frame);
						},
						OpCode::Ping => {
							println!("ping/pong");
							self.outgoing.push(WebSocketFrame::pong(&frame));
						},
						OpCode::ConnectionClose => {
							self.outgoing.push(WebSocketFrame::close_from(&frame));
						},
						_ => {}
					}
					self.interest.remove(Ready::readable());
					self.interest.insert(Ready::writable());

					match std::str::from_utf8(&frame.payload) {
						Ok(msg) => Some(msg.to_string()),
						Err(e) => None
					}
				},
				Err(e) => {
					println!("error while reading frame {}", e);
					None
				}
			}
		}

		fn read_handshake(&mut self) -> Option<String> {
			loop {
				let mut buf = [0; 2048];
				match self.socket.read(&mut buf) {
					Err(e) => {
						println!("Error while reading socket: {:?}", e);
						return None
					},
					Ok(0) => break,
					Ok(len) => {
						let is_upgrade = if let ClientState::AwaitingHandshake(ref parser_state) = self.state {
							let mut parser = parser_state.borrow_mut();
							parser.parse(&buf);
							parser.is_upgrade()
						} else { false };

						if is_upgrade {
							self.state = ClientState::HandshakeResponse;
							self.interest.remove(Ready::readable());
							self.interest.insert(Ready::writable());
							break;
						}
					}
				}
			}
			None
		}

		pub fn write(&mut self) {
			match self.state {
				ClientState::HandshakeResponse => self.write_handshake(),
				ClientState::Connected => {
					let mut close_connection = false;

					println!("sending {} frames", self.outgoing.len());
					for frame in self.outgoing.iter() {
						if let Err(e) = frame.write(&mut self.socket) {
							println!("error on write: {}", e);
						}

						if frame.is_close() {
							close_connection = true;
						}
					}

					self.outgoing.clear();
					self.interest.remove(Ready::writable());
					self.interest.insert(Ready::readable());
					if close_connection {
						self.interest.insert(Ready::hup());
					}
				},
				_ => {}
			}
		}

		fn write_handshake(&mut self) {
			let headers = self.headers.borrow();
			let response_key = gen_key(&headers.get("Sec-WebSocket-Key").unwrap());

			let response = fmt::format(format_args!("HTTP/1.1 101 Switching Protocols\r\n\
													 Connection: Upgrade\r\n\
													 Sec-WebSocket-Accept: {}\r\n\
													 Upgrade: websocket\r\n\r\n", response_key));
			self.socket.write(response.as_bytes()).unwrap();
			self.state = ClientState::Connected;
			self.interest.remove(Ready::writable());
			self.interest.insert(Ready::readable());
		}

		pub fn new(socket: TcpStream) -> WebSocketClient {
			let headers = Rc::new(RefCell::new(HashMap::new()));
			WebSocketClient {
				socket: socket,
				headers: headers.clone(),

				interest: Ready::readable(),
				state: ClientState::AwaitingHandshake(RefCell::new(Parser::request(HttpParser {
					current_key: None,
					headers: headers.clone()
				}))),
				outgoing: Vec::new()
			}
		}
	}
}