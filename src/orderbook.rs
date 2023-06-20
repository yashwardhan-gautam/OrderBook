use serde::Deserialize;
use serde_json::Value;
use std::error::Error;

#[derive(Debug, Deserialize)]
pub struct OrderBook {
    bids: Vec<(f64, f64)>,
    asks: Vec<(f64, f64)>,
}

pub async fn get_bitstamp_orderbook(symbol: &str, depth: u32) -> Result<OrderBook, Box<dyn Error>> {
    let symbol = symbol.to_lowercase();

    let url = format!("https://www.bitstamp.net/api/v2/order_book/{}", symbol);

    let http_response = reqwest::get(&url).await?;

    // Check if the response was successful
    if http_response.status().is_success() {
        let json_data: Value = http_response.json().await?;
        let order_book = parse_order_book(&json_data, depth)?;
        Ok(order_book)
    } else {
        Err(format!(
            "HTTP request failed with status code: {}",
            http_response.status()
        )
        .into())
    }
}

pub async fn get_binance_orderbook(symbol: &str, depth: u32) -> Result<OrderBook, Box<dyn Error>> {
    // Binance uses the instruments in uppercase, whereas bitstamp uses it in lowercase
    let symbol = symbol.to_uppercase();
    // Note that binance uses limit as an argument, whereas in bitstamp we get top 100 bids/asks
    let url = format!(
        "https://api.binance.com/api/v3/depth?symbol={}&limit={}",
        symbol, depth
    );

    let http_response = reqwest::get(&url).await?;

    // Check if the response was successful
    if http_response.status().is_success() {
        let json_data: Value = http_response.json().await?;
        let order_book = parse_order_book(&json_data, depth)?;
        Ok(order_book)
    } else {
        Err(format!(
            "HTTP request failed with status code: {}",
            http_response.status()
        )
        .into())
    }
}

fn parse_order_book(json_data: &Value, depth: u32) -> Result<OrderBook, Box<dyn Error>> {
    fn parse_entries(entries: &[Value], depth: u32) -> Vec<(f64, f64)> {
        entries
            .iter()
            .take(depth as usize)
            .filter_map(|entry| {
                let price = entry
                    .get(0)
                    .and_then(Value::as_str)
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(0.0);
                let quantity = entry
                    .get(1)
                    .and_then(Value::as_str)
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(0.0);
                Some((price, quantity))
            })
            .collect()
    }

    let bids = json_data["bids"].as_array().ok_or("Invalid 'bids' field")?;
    let asks = json_data["asks"].as_array().ok_or("Invalid 'asks' field")?;

    Ok(OrderBook {
        bids: parse_entries(bids, depth),
        asks: parse_entries(asks, depth),
    })
}

pub fn print_order_book(order_book: &OrderBook, depth: usize) {
    println!(
        "{:<8} {:<15} {:<17} | {:<17} {:<15}",
        "Depth", "BidVolume", "BidPrice", "AskPrice", "AskVolume"
    );
    for i in 0..depth {
        let bid = order_book.bids.get(i);
        let ask = order_book.asks.get(i);

        if let Some(bid) = bid {
            let bid_volume = format!("{:.8}", bid.1);
            let bid_price = format!("{:.8}", bid.0);

            if let Some(ask) = ask {
                let ask_price = format!("{:.8}", ask.0);
                let ask_volume = format!("{:.8}", ask.1);

                println!(
                    "{:<8} {:<15} {:<17} | {:<17} {:<15}",
                    i + 1,
                    bid_volume,
                    bid_price,
                    ask_price,
                    ask_volume
                );
            }
        }
    }
    println!(); // Append an empty line
}
