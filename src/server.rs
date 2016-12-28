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

					self.clients.insert(new_token, WebSocketClient::new(client_socket, self.token_counter));
					event_loop.register(&self.clients[&new_token].socket,
										new_token, Ready::readable(),
										PollOpt::edge() | PollOpt::oneshot()).unwrap();
				}
				token => {
					let mut message: String = String::new();
					{
						let mut client = self.clients.get_mut(&token).unwrap();
						if let Some(msg) = client.read() {
								println!("Received message: {}", msg);
								message = msg;
						}
						event_loop.reregister(&client.socket, token, client.interest,
											  PollOpt::edge() | PollOpt::oneshot()).unwrap();
					}
					if !message.is_empty() {
						self.broadcast(message);
					}
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

impl WebSocketServer {
	fn broadcast(&mut self, s: String) {
		for client in self.clients.values_mut() {
			client.push_message(s.clone());
			client.write();
		}
	}
}