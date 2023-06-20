mod orderbook;
use std::env;
use std::error::Error;

use orderbook::{get_bitstamp_orderbook, get_binance_orderbook, print_order_book};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let symbol = env::args()
        .nth(1)
        .expect("Please provide a symbol argument");
    let depth = env::args()
        .nth(2)
        .and_then(|arg| arg.parse::<u32>().ok())
        .unwrap_or(10);

    let symbol_clone = symbol.clone(); // Clone the symbol

    let binance_task = tokio::spawn(async move {
        match get_binance_orderbook(&symbol_clone, depth).await {
            Ok(order_book) => {
                println!("Binance Order Book:");
                print_order_book(&order_book, depth as usize);
            }
            Err(err) => {
                println!("Binance Error: {}", err);
            }
        }
    });

    let bitstamp_task = tokio::spawn(async move {
        match get_bitstamp_orderbook(&symbol, depth).await {
            Ok(order_book) => {
                println!("Bitstamp Order Book:");
                print_order_book(&order_book, depth as usize);
            }
            Err(err) => {
                println!("Bitstamp Error: {}", err);
            }
        }
    });

    // Wait for both tasks to complete
    binance_task.await?;
    bitstamp_task.await?;

    Ok(())
}

