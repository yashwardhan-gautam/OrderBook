mod orderbook_helper;
use orderbook_helper::{
    binance_connect, bitstamp_connect, merge_orderbooks, print_orderbook, process_message,
    OrderBook,
};

pub mod orderbook {
    tonic::include_proto!("orderbook");
}

use orderbook::{Level, Summary};

#[allow(dead_code)]
fn print_summary(summary: &Summary) {
    println!("Spread: {}", summary.spread);

    println!("Bids:");
    for bid in &summary.bids {
        println!("Exchange: {}", bid.exchange);
        println!("Price: {}", bid.price);
        println!("Amount: {}", bid.amount);
        println!();
    }

    println!("Asks:");
    for ask in &summary.asks {
        println!("Exchange: {}", ask.exchange);
        println!("Price: {}", ask.price);
        println!("Amount: {}", ask.amount);
        println!();
    }
}

fn orderbook_to_summary(orderbook: &OrderBook) -> Summary {
    let mut summary = Summary::default();
    summary.spread = orderbook.spread;

    summary.bids = orderbook
        .bids
        .iter()
        .map(|level| {
            let mut summary_level = Level::default();
            summary_level.exchange = level.exchange.clone();
            summary_level.price = level.price;
            summary_level.amount = level.amount;
            summary_level
        })
        .collect();

    summary.asks = orderbook
        .asks
        .iter()
        .map(|level| {
            let mut summary_level = Level::default();
            summary_level.exchange = level.exchange.clone();
            summary_level.price = level.price;
            summary_level.amount = level.amount;
            summary_level
        })
        .collect();

    summary
}

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

        let _summary = orderbook_to_summary(&merged_orderbook);
    }
}

#[tokio::main]
async fn main() {
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
