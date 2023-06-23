use serde_json::json;
use tokio::runtime::Runtime;
use tungstenite::{connect, Message};
use url::Url;

#[allow(dead_code)]
async fn subscribe_to_binance_stream(symbol: String, depth: u32) {
    // WebSocket server URL
    let url = Url::parse("wss://stream.binance.com:9443/ws").expect("Failed to parse URL");

    // Connect to the WebSocket server
    let (mut socket, _) = connect(url).expect("Failed to connect");

    // Construct the message
    let message = json!({
        "method": "SUBSCRIBE",
        "params": [
            format!("{}@depth{}", symbol, depth)
        ],
        "id": 1
    });

    // Connect to the WebSocket server
    let message_json = serde_json::to_string(&message).expect("Failed to serialize message");

    // Send the message as a text frame
    socket
        .write_message(Message::Text(message_json.into()))
        .expect("Failed to send message");

    // Receive and handle messages from the WebSocket server
    loop {
        let msg = socket.read_message().expect("Failed to receive message");
        println!("Received message from Binance: {:#?}", msg);
    }
}

#[allow(dead_code)]
async fn subscribe_to_bitstamp_stream(symbol: String, depth: u32) {
    // WebSocket server URL
    let url = Url::parse("wss://ws.bitstamp.net/").expect("Failed to parse URL");

    // Connect to the WebSocket server
    let (mut socket, _) = connect(url).expect("Failed to connect");

    // Construct the message
    let channel = format!("detail_order_book_{}", symbol);
    let message = format!(
        r#"
        {{
            "event": "bts:subscribe",
            "data": {{
                "channel": "{}"
            }}
        }}
        "#,
        channel
    );

    println!("message: {:#?}", message);

    // Send the message as a text frame
    socket
        .write_message(Message::Text(message.into()))
        .expect("Failed to send message");
    // Receive and handle messages from the WebSocket server
    loop {
        let msg = socket.read_message().expect("Failed to receive message");
        println!("Received message from Bitstamp: {:#?}", msg);
    }
}

fn main() {
    // Parse command-line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: cargo run -- <symbol> [depth]");
        return;
    }
    let symbol = args[1].clone();
    let depth = args.get(2).and_then(|d| d.parse().ok()).unwrap_or(10);

    // Create a tokio runtime
    let rt = Runtime::new().expect("Failed to create Tokio runtime");

    // Spawn the tasks for subscribing to the streams concurrently
    rt.spawn(subscribe_to_binance_stream(symbol.clone(), depth));
    rt.spawn(subscribe_to_bitstamp_stream(symbol.clone(), depth));

    // Run the tokio runtime
    rt.block_on(async {
        // Keep the main thread alive
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for Ctrl+C");
    });
}
