pub use self::server::WebSocketServer;

mod server {
	extern crate mio;

	use mio::*;
	use mio::deprecated::{Handler, EventLoop};
	use mio::tcp::*;
	use std::collections::HashMap;

	use client::WebSocketClient;

	pub struct WebSocketServer {
		pub socket: TcpListener,
		pub clients: HashMap<Token, WebSocketClient>,
		pub token_counter: usize
	}

	const SERVER_TOKEN: Token = Token(0);

	impl Handler for WebSocketServer {
		type Timeout = usize;
		type Message = ();

		fn ready(&mut self, event_loop: &mut EventLoop<WebSocketServer>,
				 token: Token, events: Ready)
		{
			if events.is_readable()  {
				match token {
					SERVER_TOKEN => {
						let client_socket = match self.socket.accept() {
							Err(e) => {
								println!("Accept error: {}", e);
								return;
							},
							Ok((sock, _)) => sock
						};

						println!("Now serving {} clients", &self.token_counter);

						self.token_counter += 1;
						let new_token = Token(self.token_counter);

						self.clients.insert(new_token, WebSocketClient::new(client_socket));
						event_loop.register(&self.clients[&new_token].socket,
											new_token, Ready::readable(),
											PollOpt::edge() | PollOpt::oneshot()).unwrap();
					}
					token => {
						let mut client = self.clients.get_mut(&token).unwrap();
						match client.read() {
							Some(msg) => println!("Received message: {}", msg),
							None => {}
						};
						event_loop.reregister(&client.socket, token, client.interest,
											  PollOpt::edge() | PollOpt::oneshot()).unwrap();
					}
				}
			}

			if events.is_writable() {
				let mut client = self.clients.get_mut(&token).unwrap();
				client.write();
				event_loop.reregister(&client.socket, token, client.interest,
									  PollOpt::edge() | PollOpt::oneshot()).unwrap();
			}

			if events.is_hup() {
				let client = self.clients.remove(&token).unwrap();
				client.socket.shutdown(Shutdown::Both);
				event_loop.deregister(&client.socket);
				self.token_counter -= 1;
				println!("disconnected from client");
			}
		}
	}
}