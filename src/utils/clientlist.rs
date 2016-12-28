use std::collections::HashMap;
use client::WebSocketClient;

pub struct ClientList {
	clients: Vec<WebSocketClient>,
	client_count: u8
}

impl ClientList {
	pub fn broadcast(s: String) -> Result<(),String> {
		Ok(())
	}

	pub fn new() -> ClientList {
		ClientList {
			clients: Vec::new(),
			client_count: 0
		}
	}
}