use tungstenite::{connect, Message};
use url::Url;

fn subscribe_to_stream(symbol: &str, depth: u32) {
    // WebSocket server URL
    let url = Url::parse("wss://stream.binance.com:9443/ws").expect("Failed to parse URL");

    // Connect to the WebSocket server
    let (mut socket, _) = connect(url).expect("Failed to connect");

    // Construct the message
    let message = format!(
        r#"
        {{
            "method": "SUBSCRIBE",
            "params": [
                "{}@depth{}"
            ],
            "id": 1
        }}
        "#,
        symbol, depth
    );

    println!("{:#?}", message);
    // Send the message as a text frame
    socket
        .write_message(Message::Text(message.into()))
        .expect("Failed to send message");

    // Receive and handle messages from the WebSocket server
    loop {
        let msg = socket.read_message().expect("Failed to receive message");
        println!("Received message: {:#?}", msg);
        // Add your own logic here to handle the received messages

        // Wait for one minute before listening to the next stream
        // thread::sleep(Duration::from_secs(60));
    }
}

fn main() {
    // Parse command-line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: cargo run -- <symbol> [depth]");
        return;
    }
    let symbol = &args[1];
    let depth = args.get(2).and_then(|d| d.parse().ok()).unwrap_or(10);

    // Subscribe to the stream
    subscribe_to_stream(symbol, depth);
}
