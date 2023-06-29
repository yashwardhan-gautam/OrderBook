pub mod orderbook {
    tonic::include_proto!("orderbook");
}
use orderbook::orderbook_aggregator_client::OrderbookAggregatorClient;
use orderbook::{Empty, Summary};
use tonic::Request;

fn print_summary(summary: &Summary) {
    println!("Spread: {:#?}", summary.spread);
    println!(
        "{:<6} {:<12} {:<16} {:<12} | {:<12} {:<16} {:<12}",
        "Depth", "BidExchange", "BidVolume", "BidPrice", "AskPrice", "AskVolume", "AskExchange"
    );

    let max_levels = summary.bids.len().max(summary.asks.len());

    for i in 0..max_levels {
        let bid = summary.bids.get(i);
        let ask = summary.asks.get(i);

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "http://localhost:50051";

    let mut client = OrderbookAggregatorClient::connect(addr).await?;

    let request = Request::new(Empty {});
    let mut stream = client.book_summary(request).await?.into_inner();

    while let Some(summary) = stream.message().await? {
        println!("Orderbook received: ");
        print_summary(&summary);
    }

    Ok(())
}
