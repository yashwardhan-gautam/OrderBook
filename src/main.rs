use serde::Deserialize;
use serde_json::{json, Value};
use tungstenite::{connect, Message};
use url::Url;

#[derive(Debug, Deserialize)]
pub struct Level {
    exchange: String,
    #[serde(rename = "p")]
    price: f64,
    #[serde(rename = "q")]
    amount: f64,
}

impl Clone for Level {
    fn clone(&self) -> Self {
        Level {
            exchange: self.exchange.clone(),
            price: self.price,
            amount: self.amount,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct OrderBook {
    #[serde(rename = "b")]
    bids: Vec<Level>,
    #[serde(rename = "a")]
    asks: Vec<Level>,
    #[serde(rename = "s")]
    spread: f64,
}

impl OrderBook {
    fn new() -> OrderBook {
        OrderBook {
            bids: Vec::new(),
            asks: Vec::new(),
            spread: 0.0,
        }
    }
}

fn print_order_book(order_book: &OrderBook) {
    println!("Spread: {:#?}", order_book.spread);
    println!(
        "{:<6} {:<12} {:<16} {:<12} | {:<12} {:<16} {:<12}",
        "Depth", "BidExchange", "BidVolume", "BidPrice", "AskPrice", "AskVolume", "AskExchange"
    );

    let max_levels = order_book.bids.len().max(order_book.asks.len());

    for i in 0..max_levels {
        let bid = order_book.bids.get(i);
        let ask = order_book.asks.get(i);

        let bid_exchange = bid.map(|b| b.exchange.clone()).unwrap_or("".to_string());
        let bid_amount = bid.map(|b| b.amount.to_string()).unwrap_or("".to_string());
        let bid_price = bid.map(|b| b.price.to_string()).unwrap_or("".to_string());

        let ask_price = ask.map(|a| a.price.to_string()).unwrap_or("".to_string());
        let ask_amount = ask.map(|a| a.amount.to_string()).unwrap_or("".to_string());
        let ask_exchange = ask.map(|a| a.exchange.clone()).unwrap_or("".to_string());

        println!(
            "{:<6} {:<12} {:<16} {:<12} | {:<12} {:<16} {:<12}",
            format!("[{}]", i + 1),
            bid_exchange,
            bid_amount,
            bid_price,
            ask_price,
            ask_amount,
            ask_exchange
        );
    }

    println!();
}

fn sort_and_select_levels(levels: &[Level], depth: usize, ascending: bool) -> Vec<Level> {
    let mut sorted_levels = levels.to_vec();

    sorted_levels.sort_by(|a, b| {
        if a.price == b.price {
            // If prices are the same, sort by descending order of amount
            b.amount.partial_cmp(&a.amount).unwrap()
        } else if ascending {
            // Sort by ascending order of price
            a.price.partial_cmp(&b.price).unwrap()
        } else {
            // Sort by descending order of price
            b.price.partial_cmp(&a.price).unwrap()
        }
    });

    if depth <= sorted_levels.len() {
        sorted_levels[..depth].to_vec()
    } else {
        sorted_levels
    }
}

fn process_binance_message(message_text: &str, depth: usize) -> Option<OrderBook> {
    if let Ok(result) = serde_json::from_str::<Value>(message_text) {
        let bids: Vec<Level> = if let Some(bids) = result["bids"].as_array() {
            bids.iter()
                .filter_map(|bid| {
                    if let Some(price) = bid
                        .get(0)
                        .and_then(|v| v.as_str().and_then(|s| s.parse().ok()))
                    {
                        if let Some(amount) = bid
                            .get(1)
                            .and_then(|v| v.as_str().and_then(|s| s.parse().ok()))
                        {
                            return Some(Level {
                                exchange: "binance".to_string(),
                                price,
                                amount,
                            });
                        }
                    }
                    None
                })
                .collect()
        } else {
            return None; // Return early if bids array is missing
        };

        let asks: Vec<Level> = if let Some(asks) = result["asks"].as_array() {
            asks.iter()
                .filter_map(|ask| {
                    if let Some(price) = ask
                        .get(0)
                        .and_then(|v| v.as_str().and_then(|s| s.parse().ok()))
                    {
                        if let Some(amount) = ask
                            .get(1)
                            .and_then(|v| v.as_str().and_then(|s| s.parse().ok()))
                        {
                            return Some(Level {
                                exchange: "binance".to_string(),
                                price,
                                amount,
                            });
                        }
                    }
                    None
                })
                .collect()
        } else {
            return None; // Return early if asks array is missing
        };

        let spread = match (bids.first(), asks.first()) {
            (Some(first_bid), Some(first_ask)) => first_ask.price - first_bid.price,
            _ => 0.0, // Default value in case bids or asks are empty
        };

        let selected_bids = sort_and_select_levels(&bids, depth, false);
        let selected_asks = sort_and_select_levels(&asks, depth, true);

        // Return the selected bids and asks along with the actual number of levels selected
        let order_book = OrderBook {
            bids: selected_bids.to_vec(),
            asks: selected_asks.to_vec(),
            spread,
        };

        // println!("Binance Order Book {:#?}", order_book);
        println!("Binance Order Book: ");
        print_order_book(&order_book);

        Some(order_book)
    } else {
        None // Return early if JSON deserialization fails
    }
}

fn process_bitstamp_message(message_text: &str, depth: usize) -> Option<OrderBook> {
    if let Ok(mut result) = serde_json::from_str::<Value>(message_text) {
        result = result["data"].clone();
        let bids: Vec<Level> = if let Some(bids) = result["bids"].as_array() {
            bids.iter()
                .filter_map(|bid| {
                    if let Some(price) = bid
                        .get(0)
                        .and_then(|v| v.as_str().and_then(|s| s.parse().ok()))
                    {
                        if let Some(amount) = bid
                            .get(1)
                            .and_then(|v| v.as_str().and_then(|s| s.parse().ok()))
                        {
                            return Some(Level {
                                exchange: "bitstamp".to_string(),
                                price,
                                amount,
                            });
                        }
                    }
                    None
                })
                .collect()
        } else {
            return None; // Return early if bids array is missing
        };

        let asks: Vec<Level> = if let Some(asks) = result["asks"].as_array() {
            asks.iter()
                .filter_map(|ask| {
                    if let Some(price) = ask
                        .get(0)
                        .and_then(|v| v.as_str().and_then(|s| s.parse().ok()))
                    {
                        if let Some(amount) = ask
                            .get(1)
                            .and_then(|v| v.as_str().and_then(|s| s.parse().ok()))
                        {
                            return Some(Level {
                                exchange: "bitstamp".to_string(),
                                price,
                                amount,
                            });
                        }
                    }
                    None
                })
                .collect()
        } else {
            return None; // Return early if asks array is missing
        };

        let spread = match (bids.first(), asks.first()) {
            (Some(first_bid), Some(first_ask)) => first_ask.price - first_bid.price,
            _ => 0.0, // Default value in case bids or asks are empty
        };

        let selected_bids = sort_and_select_levels(&bids, depth, false);
        let selected_asks = sort_and_select_levels(&asks, depth, true);

        // Return the selected bids and asks along with the actual number of levels selected
        let order_book = OrderBook {
            bids: selected_bids.to_vec(),
            asks: selected_asks.to_vec(),
            spread,
        };

        // println!("Bitstamp Order Book {:#?}", order_book);
        println!("Bitstamp Order Book: ");
        print_order_book(&order_book);

        Some(order_book)
    } else {
        None // Return early if JSON deserialization fails
    }
}

fn process_message(exchange: &str, message_text: &str, depth: usize) -> Option<OrderBook> {
    println!("Processing message for exchange: {}", exchange);
    match exchange {
        "binance" => process_binance_message(message_text, depth),
        "bitstamp" => process_bitstamp_message(message_text, depth),
        _ => {
            println!("Invalid exchange: {}", exchange);
            None
        }
    }
}

fn subscribe_to_streams(symbol: &str, depth: u32) {
    // Binance WebSocket server URL
    let binance_url =
        Url::parse("wss://stream.binance.com:9443/ws").expect("Failed to parse Binance URL");

    // Bitstamp WebSocket server URL
    let bitstamp_url = Url::parse("wss://ws.bitstamp.net/").expect("Failed to parse Bitstamp URL");

    // Connect to the Binance WebSocket server
    let (mut binance_socket, _) = connect(binance_url).expect("Failed to connect to Binance");

    // Connect to the Bitstamp WebSocket server
    let (mut bitstamp_socket, _) = connect(bitstamp_url).expect("Failed to connect to Bitstamp");

    // Construct the Binance subscription message
    let binance_message = json!({
        "method": "SUBSCRIBE",
        "params": [
            format!("{}@depth{}", symbol, depth)
        ],
        "id": 1
    });

    // Construct the Bitstamp subscription message
    let bitstamp_channel = format!("detail_order_book_{}", symbol);
    let bitstamp_message = format!(
        r#"
        {{
            "event": "bts:subscribe",
            "data": {{
                "channel": "{}"
            }}
        }}
        "#,
        bitstamp_channel
    );

    // Send the subscription messages as text frames
    binance_socket
        .write_message(Message::Text(
            serde_json::to_string(&binance_message).unwrap().into(),
        ))
        .expect("Failed to send Binance subscription message");
    bitstamp_socket
        .write_message(Message::Text(bitstamp_message.into()))
        .expect("Failed to send Bitstamp subscription message");

    // Receive and handle messages from both WebSocket servers
    let mut binance_orderbook = OrderBook::new();
    let mut bitstamp_orderbook = OrderBook::new();

    loop {
        let binance_msg = binance_socket
            .read_message()
            .expect("Failed to receive message from Binance");
        if let Ok(message_text) = binance_msg.to_text() {
            if let Some(orderbook) = process_message("binance", message_text, depth as usize) {
                binance_orderbook = orderbook;
            }
        }

        let bitstamp_msg = bitstamp_socket
            .read_message()
            .expect("Failed to receive message from Bitstamp");
        if let Ok(message_text) = bitstamp_msg.to_text() {
            if let Some(orderbook) = process_message("bitstamp", message_text, depth as usize) {
                bitstamp_orderbook = orderbook;
            }
        }
        println!("Binance OrderBook");
        print_order_book(&binance_orderbook);
        println!("Bitstamp OrderBook");
        print_order_book(&bitstamp_orderbook);

        // merge orderbooks(binance_orderbook, bitstamp_orderbook, depth);
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
