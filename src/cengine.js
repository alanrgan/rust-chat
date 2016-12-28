// TODO: implement a connection interface
$(document).ready(function() {
	var ws = null;
	var connected = false;
	var connection = null;

	$("#server_connect").click(function(event) {
		if(!ws || connection.ping(ws) == false) {
			ws = new WebSocket("ws://127.0.0.1:10000");
			ws.onmessage = function(event) {
				$("#text").append(event.data);
			}
		}
	});

	$("#message_submit").click(function(event) {
		sendMessage();
	});

	$("#message_input").keypress(function(e) {
		if(e.which == 13) {
			sendMessage();
			$(this).val('');
		}
	});

	function sendMessage() {
		if(!ws) return;
		var msg = $("#message_input").val();
		if(msg !== "")
			ws.send(msg);
	}
});