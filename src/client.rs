pub mod orderbook {
    tonic::include_proto!("orderbook");
}

use orderbook::orderbook_aggregator_client::OrderbookAggregatorClient;
use orderbook::Empty;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "http://localhost:50051";

    let mut client = OrderbookAggregatorClient::connect(addr).await?;
    loop {
        let request = tonic::Request::new(Empty {});
        let mut stream = client.book_summary(request).await?.into_inner();
        while let Some(summary) = stream.message().await? {
            // Process the received summary here
            println!("Received summary: {:?}", summary);
        }
    }
}
