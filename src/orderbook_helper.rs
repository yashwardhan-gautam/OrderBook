use serde::Deserialize;
use serde_json::Value;
use tungstenite::client::AutoStream;
use tungstenite::{connect, Message, WebSocket};
use url::Url;

#[derive(Debug, Deserialize)]
pub struct Level {
    pub exchange: String,
    pub price: f64,
    pub amount: f64,
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

#[derive(Debug, Default, Deserialize)]
pub struct OrderBook {
    pub bids: Vec<Level>,
    pub asks: Vec<Level>,
    pub spread: f64,
}

impl OrderBook {
    pub fn new() -> OrderBook {
        OrderBook {
            bids: Vec::new(),
            asks: Vec::new(),
            spread: 0.0,
        }
    }
}

pub fn print_orderbook(orderbook: &OrderBook) {
    println!("Spread: {:#?}", orderbook.spread);
    println!(
        "{:<6} {:<12} {:<16} {:<12} | {:<12} {:<16} {:<12}",
        "Depth", "BidExchange", "BidVolume", "BidPrice", "AskPrice", "AskVolume", "AskExchange"
    );

    let max_levels = orderbook.bids.len().max(orderbook.asks.len());

    for i in 0..max_levels {
        let bid = orderbook.bids.get(i);
        let ask = orderbook.asks.get(i);

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

fn sort_and_trim_levels(levels: &[Level], depth: usize, ascending: bool) -> Vec<Level> {
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

pub fn process_message(message_text: &str, exchange: &str, depth: usize) -> Option<OrderBook> {
    if let Ok(result) = serde_json::from_str::<Value>(message_text) {
        // for bitstamp the "bids" and "asks" are inside "data" key
        // whereas for binance we can directly access the "bids" and "asks"
        let mut data = result.get("data").cloned();
        if data.is_none() {
            data = Some(result);
        }

        if let Some(data) = data {
            let bids: Vec<Level> = if let Some(bids) = data["bids"].as_array() {
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
                                    exchange: exchange.to_string(),
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

            let asks: Vec<Level> = if let Some(asks) = data["asks"].as_array() {
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
                                    exchange: exchange.to_string(),
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
                (Some(first_bid), Some(first_ask)) => first_bid.price - first_ask.price,
                _ => 0.0, // Default value in case bids or asks are empty
            };

            let selected_bids = sort_and_trim_levels(&bids, depth, false);
            let selected_asks = sort_and_trim_levels(&asks, depth, true);

            // Return the selected bids and asks along with the actual number of levels selected
            let orderbook = OrderBook {
                bids: selected_bids.to_vec(),
                asks: selected_asks.to_vec(),
                spread,
            };

            Some(orderbook)
        } else {
            None // Return early if data is None
        }
    } else {
        None // Return early if JSON deserialization fails
    }
}

pub fn merge_orderbooks(
    binance_orderbook: &OrderBook,
    bitstamp_orderbook: &OrderBook,
    depth: usize,
) -> OrderBook {
    let mut merged_bids = binance_orderbook.bids.clone();
    merged_bids.extend(bitstamp_orderbook.bids.iter().cloned());

    let mut merged_asks = binance_orderbook.asks.clone();
    merged_asks.extend(bitstamp_orderbook.asks.iter().cloned());

    let sorted_bids = sort_and_trim_levels(&merged_bids, depth, false);
    let sorted_asks = sort_and_trim_levels(&merged_asks, depth, true);

    let spread = match (sorted_bids.first(), sorted_asks.first()) {
        (Some(first_bid), Some(first_ask)) => first_bid.price - first_ask.price,
        _ => 0.0,
    };

    OrderBook {
        bids: sorted_bids,
        asks: sorted_asks,
        spread,
    }
}

pub fn binance_connect(symbol: &str, depth: u32) -> WebSocket<AutoStream> {
    // Binance WebSocket server URL
    let binance_url =
        Url::parse("wss://stream.binance.com:9443/ws").expect("Failed to parse Binance URL");

    // Connect to the Binance WebSocket server
    let (mut binance_socket, _) = connect(binance_url).expect("Failed to connect to Binance");

    // Construct the Binance subscription message
    let binance_message = format!(
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

    // Send the subscription message as a text frame
    binance_socket
        .write_message(Message::Text(binance_message.into()))
        .expect("Failed to send Binance subscription message");

    // Read the first message from the socket
    let first_message = binance_socket
        .read_message()
        .expect("Failed to receive the first message from Binance");

    // Verify that the first message is a text frame
    if let Message::Text(first_message_text) = first_message {
        if first_message_text == "{\"result\":null,\"id\":1}" {
            println!("Connected with Binance Stream successfully");
        } else {
            panic!("Failed to connect with Binance Stream");
        }
    } else {
        panic!("Received an unexpected message type from Binance");
    }

    binance_socket
}

pub fn bitstamp_connect(symbol: &str) -> WebSocket<AutoStream> {
    // Bitstamp WebSocket server URL
    let bitstamp_url = Url::parse("wss://ws.bitstamp.net/").expect("Failed to parse Bitstamp URL");

    // Connect to the Bitstamp WebSocket server
    let (mut bitstamp_socket, _) = connect(bitstamp_url).expect("Failed to connect to Bitstamp");

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
    bitstamp_socket
        .write_message(Message::Text(bitstamp_message.into()))
        .expect("Failed to send Bitstamp subscription message");

    // Read the first message from the socket
    let first_message = bitstamp_socket
        .read_message()
        .expect("Failed to receive the first message from Bitstamp");

    if let Message::Text(first_message_text) = first_message {
        if first_message_text.as_str() == &format!("{{\"event\":\"bts:subscription_succeeded\",\"channel\":\"detail_order_book_{}\",\"data\":{{}}}}", symbol) {
            println!("Connected with Bitstamp Stream successfully");
        } else {
            panic!("Failed to connect with Bitstamp Stream");
        }
    } else {
        panic!("Received an unexpected message type from Bitstamp");
    }

    bitstamp_socket
}
