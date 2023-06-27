pub mod orderbook {
    tonic::include_proto!("orderbook");
}

use orderbook::orderbook_aggregator_client::OrderbookAggregatorClient;
use orderbook::{Empty, Summary};
use tonic::transport::Channel;

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

async fn read_book_summaries(
    server_address: &'static str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the server
    let channel = Channel::from_static(server_address).connect().await?;

    // Create a gRPC client
    let mut client = OrderbookAggregatorClient::new(channel);

    // Create an empty request
    let request = tonic::Request::new(Empty {});

    // Call the server's book_summary method to receive a stream of book summaries
    let mut stream = client.book_summary(request).await?.into_inner();

    // Read book summaries from the stream
    while let Some(summary) = stream.message().await? {
        // Process the received summary
        println!("Received book summary:");
        print_summary(&summary);
        println!("-------------------------");
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server_address = "http://localhost:50051";

    read_book_summaries(server_address).await?;

    Ok(())
}
