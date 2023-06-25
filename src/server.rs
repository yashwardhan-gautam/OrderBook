mod orderbook;
use orderbook::{
    binance_connect, bitstamp_connect, merge_orderbooks, print_orderbook, process_message,
    OrderBook,
};

fn subscribe_to_streams(symbol: &str, depth: u32) {
    let mut binance_socket = binance_connect(symbol, depth);
    let mut bitstamp_socket = bitstamp_connect(symbol);

    // Receive and handle messages from both WebSocket servers
    let mut binance_orderbook = OrderBook::new();
    let mut bitstamp_orderbook = OrderBook::new();

    loop {
        let binance_msg = binance_socket
            .read_message()
            .expect("Failed to receive message from Binance");
        if let Ok(message_text) = binance_msg.to_text() {
            if let Some(orderbook) = process_message(message_text, "binance", depth as usize) {
                binance_orderbook = orderbook;
            }
        }

        let bitstamp_msg = bitstamp_socket
            .read_message()
            .expect("Failed to receive message from Bitstamp");
        if let Ok(message_text) = bitstamp_msg.to_text() {
            if let Some(orderbook) = process_message(message_text, "bitstamp", depth as usize) {
                bitstamp_orderbook = orderbook;
            }
        }

        // println!("Binance OrderBook");
        // print_orderbook(&binance_orderbook);
        // println!("Bitstamp OrderBook");
        // print_orderbook(&bitstamp_orderbook);

        let merged_orderbook =
            merge_orderbooks(&binance_orderbook, &bitstamp_orderbook, depth as usize);
        println!("Merged OrderBook");
        print_orderbook(&merged_orderbook);
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

    subscribe_to_streams(symbol, depth);
}
