use serde::Deserialize;
use serde_json::Value;
use std::error::Error;
use tungstenite::client::AutoStream;
use tungstenite::{connect, Message, WebSocket};
use url::Url;

#[derive(Debug, Deserialize, Clone)]
pub struct PriceAmountLevel {
    pub exchange: String,
    pub price: f64,
    pub amount: f64,
}

#[derive(Debug, Default, Deserialize, Clone)]
pub struct OrderBook {
    pub bids: Vec<PriceAmountLevel>,
    pub asks: Vec<PriceAmountLevel>,
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

fn sort_and_trim_levels(
    levels: &[PriceAmountLevel],
    depth: usize,
    ascending: bool,
) -> Vec<PriceAmountLevel> {
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
            let bids: Vec<PriceAmountLevel> = if let Some(bids) = data["bids"].as_array() {
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
                                return Some(PriceAmountLevel {
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

            let asks: Vec<PriceAmountLevel> = if let Some(asks) = data["asks"].as_array() {
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
                                return Some(PriceAmountLevel {
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

pub async fn binance_connect(
    symbol: &str,
    depth: u32,
) -> Result<WebSocket<AutoStream>, Box<dyn Error>> {
    // Binance WebSocket server URL
    let binance_url =
        Url::parse("wss://stream.binance.com:9443/ws").expect("Failed to parse Binance URL");

    // Connect to the Binance WebSocket server
    let (mut binance_socket, _) = connect(binance_url).expect("Failed to connect to Binance");

    // Construct the Binance subscription message
    // binance support two update speeds - 1000ms or 100ms
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
    let connection_message = binance_socket
        .read_message()
        .expect("Failed to receive the first message from Binance");

    // Verify that the first message is a text frame
    if let Message::Text(connection_message_text) = connection_message {
        if connection_message_text == "{\"result\":null,\"id\":1}" {
            println!("Connected with Binance Stream successfully");
        } else {
            panic!("Failed to connect with Binance Stream");
        }
    } else {
        panic!("Received an unexpected message type from Binance");
    }

    Ok(binance_socket)
}

pub async fn bitstamp_connect(symbol: &str) -> Result<WebSocket<AutoStream>, Box<dyn Error>> {
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
    let connection_message = bitstamp_socket
        .read_message()
        .expect("Failed to receive the first message from Bitstamp");

    if let Message::Text(connection_message_text) = connection_message {
        if connection_message_text.as_str()
            == &format!(
                "{{\"event\":\"bts:subscription_succeeded\",\"channel\":\"detail_order_book_{}\",\"data\":{{}}}}",
                symbol
            )
        {
            println!("Connected with Bitstamp Stream successfully");
        } else {
            panic!("Failed to connect with Bitstamp Stream");
        }
    } else {
        panic!("Received an unexpected message type from Bitstamp");
    }

    Ok(bitstamp_socket)
}

// Unit test cases
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print_orderbook() {
        let orderbook = OrderBook {
            bids: vec![
                PriceAmountLevel {
                    exchange: "exchange1".to_string(),
                    price: 10.0,
                    amount: 1.0,
                },
                PriceAmountLevel {
                    exchange: "exchange2".to_string(),
                    price: 9.5,
                    amount: 2.0,
                },
            ],
            asks: vec![
                PriceAmountLevel {
                    exchange: "exchange3".to_string(),
                    price: 11.0,
                    amount: 0.8,
                },
                PriceAmountLevel {
                    exchange: "exchange4".to_string(),
                    price: 11.5,
                    amount: 0.7,
                },
            ],
            spread: 0.5,
        };

        print_orderbook(&orderbook);
    }

    #[test]
    fn test_sort_and_trim_levels() {
        let levels = vec![
            PriceAmountLevel {
                exchange: "exchange1".to_string(),
                price: 10.0,
                amount: 1.0,
            },
            PriceAmountLevel {
                exchange: "exchange2".to_string(),
                price: 9.5,
                amount: 2.0,
            },
            PriceAmountLevel {
                exchange: "exchange3".to_string(),
                price: 11.0,
                amount: 0.8,
            },
        ];

        let sorted_levels = sort_and_trim_levels(&levels, 2, true);

        assert_eq!(sorted_levels.len(), 2);
        assert_eq!(sorted_levels[0].price, 9.5);
        assert_eq!(sorted_levels[1].price, 10.0);
    }

    #[test]
    fn test_process_message() {
        let message_text = r#"
            {
                "data": {
                    "bids": [
                        [ "10.0", "1.0" ],
                        [ "9.5", "2.0" ]
                    ],
                    "asks": [
                        [ "11.0", "0.8" ],
                        [ "11.5", "0.7" ]
                    ]
                }
            }
        "#;

        let exchange = "exchange1";
        let depth = 2;

        let orderbook = process_message(message_text, exchange, depth).unwrap();

        assert_eq!(orderbook.bids.len(), 2);

        assert_eq!(orderbook.bids[0].exchange, "exchange1");
        assert_eq!(orderbook.bids[0].price, 10.0);
        assert_eq!(orderbook.bids[0].amount, 1.0);

        assert_eq!(orderbook.bids[1].exchange, "exchange1");
        assert_eq!(orderbook.bids[1].price, 9.5);
        assert_eq!(orderbook.bids[1].amount, 2.0);

        // Assert the ask levels
        assert_eq!(orderbook.asks.len(), 2);

        assert_eq!(orderbook.asks[0].exchange, "exchange1");
        assert_eq!(orderbook.asks[0].price, 11.0);
        assert_eq!(orderbook.asks[0].amount, 0.8);

        assert_eq!(orderbook.asks[1].exchange, "exchange1");
        assert_eq!(orderbook.asks[1].price, 11.5);
        assert_eq!(orderbook.asks[1].amount, 0.7);

        // Assert the spread value
        assert_eq!(orderbook.spread, -1.0);
    }

    #[test]
    fn test_merge_orderbooks() {
        let binance_orderbook = OrderBook {
            bids: vec![
                PriceAmountLevel {
                    exchange: "binance".to_string(),
                    price: 10.0,
                    amount: 1.0,
                },
                PriceAmountLevel {
                    exchange: "binance".to_string(),
                    price: 9.5,
                    amount: 2.0,
                },
            ],
            asks: vec![
                PriceAmountLevel {
                    exchange: "binance".to_string(),
                    price: 11.0,
                    amount: 0.8,
                },
                PriceAmountLevel {
                    exchange: "binance".to_string(),
                    price: 11.5,
                    amount: 0.7,
                },
            ],
            spread: 0.5,
        };

        let bitstamp_orderbook = OrderBook {
            bids: vec![
                PriceAmountLevel {
                    exchange: "bitstamp".to_string(),
                    price: 10.2,
                    amount: 0.9,
                },
                PriceAmountLevel {
                    exchange: "bitstamp".to_string(),
                    price: 9.8,
                    amount: 1.5,
                },
            ],
            asks: vec![
                PriceAmountLevel {
                    exchange: "bitstamp".to_string(),
                    price: 11.2,
                    amount: 0.6,
                },
                PriceAmountLevel {
                    exchange: "bitstamp".to_string(),
                    price: 11.8,
                    amount: 0.4,
                },
            ],
            spread: 0.6,
        };

        let depth = 3;

        let merged_orderbook = merge_orderbooks(&binance_orderbook, &bitstamp_orderbook, depth);

        assert_eq!(merged_orderbook.bids.len(), 3);
        assert_eq!(merged_orderbook.asks.len(), 3);

        // Assert the bid levels
        assert_eq!(merged_orderbook.bids[0].exchange, "bitstamp");
        assert_eq!(merged_orderbook.bids[0].price, 10.2);
        assert_eq!(merged_orderbook.bids[0].amount, 0.9);

        assert_eq!(merged_orderbook.bids[1].exchange, "binance");
        assert_eq!(merged_orderbook.bids[1].price, 10.0);
        assert_eq!(merged_orderbook.bids[1].amount, 1.0);

        assert_eq!(merged_orderbook.bids[2].exchange, "bitstamp");
        assert_eq!(merged_orderbook.bids[2].price, 9.8);
        assert_eq!(merged_orderbook.bids[2].amount, 1.5);

        // Assert the ask levels
        assert_eq!(merged_orderbook.asks[0].exchange, "binance");
        assert_eq!(merged_orderbook.asks[0].price, 11.0);
        assert_eq!(merged_orderbook.asks[0].amount, 0.8);

        assert_eq!(merged_orderbook.asks[1].exchange, "bitstamp");
        assert_eq!(merged_orderbook.asks[1].price, 11.2);
        assert_eq!(merged_orderbook.asks[1].amount, 0.6);

        assert_eq!(merged_orderbook.asks[2].exchange, "binance");
        assert_eq!(merged_orderbook.asks[2].price, 11.5);
        assert_eq!(merged_orderbook.asks[2].amount, 0.7);
    }

    #[tokio::test]
    async fn test_binance_connect() {
        let symbol = "BTCUSDT";
        let depth = 5;

        let result = binance_connect(symbol, depth).await;

        assert_eq!(result.is_ok(), true);
    }

    #[tokio::test]
    async fn test_bitstamp_connect() {
        let symbol = "btcusd";

        let result = bitstamp_connect(symbol).await;

        assert_eq!(result.is_ok(), true);
    }
}
