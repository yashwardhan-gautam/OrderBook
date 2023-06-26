mod orderbook_helper;
use orderbook_helper::{
    binance_connect, bitstamp_connect, merge_orderbooks, print_orderbook, process_message,
    OrderBook,
};

pub mod orderbook {
    tonic::include_proto!("orderbook");
}

use futures::stream::{Stream, StreamExt}; // Add this import
use orderbook::orderbook_aggregator_server::{OrderbookAggregator, OrderbookAggregatorServer};
use orderbook::{Empty, Level, Summary};
use std::pin::Pin;
use tonic::{transport::Server, Code, Request, Response, Status};

#[derive(Default)]
struct OrderbookAggregatorService {
    symbol: String,
    depth: u32,
}

#[tonic::async_trait]
impl OrderbookAggregator for OrderbookAggregatorService {
    type BookSummaryStream =
        Pin<Box<dyn Stream<Item = Result<Summary, Status>> + Send + Sync + 'static>>;

    async fn book_summary(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::BookSummaryStream>, Status> {
        let (sender, receiver) = tokio::sync::mpsc::channel(100);
        let symbol = self.symbol.clone();
        let depth = self.depth;

        let subscription_result = subscribe_to_streams(sender, &symbol, depth).await;

        if let Err(err) = subscription_result {
            return Err(Status::new(
                Code::Internal,
                format!("Error during subscription: {}", err),
            ));
        }

        let stream = tokio_stream::wrappers::ReceiverStream::new(receiver).map(
            |result: Result<Summary, ()>| {
                result.map_err(|_| Status::new(Code::Internal, "Unknown error occurred"))
            },
        );

        let response_stream: Self::BookSummaryStream = Box::pin(stream);
        Ok(Response::new(response_stream))
    }
}

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

async fn subscribe_to_streams(
    sender: tokio::sync::mpsc::Sender<Result<Summary, ()>>,
    symbol: &str,
    depth: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut binance_socket = binance_connect(symbol, depth);
    let mut bitstamp_socket = bitstamp_connect(symbol);

    // Receive and handle messages from both WebSocket servers
    let mut binance_orderbook = OrderBook::new();
    let mut bitstamp_orderbook = OrderBook::new();

    let merged_orderbook;

    // Read orderbook data only once
    if let Ok(binance_msg) = binance_socket.read_message() {
        if let Ok(message_text) = binance_msg.to_text() {
            if let Some(orderbook) = process_message(message_text, "binance", depth as usize) {
                binance_orderbook = orderbook;
            }
        }
    }

    if let Ok(bitstamp_msg) = bitstamp_socket.read_message() {
        if let Ok(message_text) = bitstamp_msg.to_text() {
            if let Some(orderbook) = process_message(message_text, "bitstamp", depth as usize) {
                bitstamp_orderbook = orderbook;
            }
        }
    }

    merged_orderbook = merge_orderbooks(&binance_orderbook, &bitstamp_orderbook, depth as usize);
    println!("Merged orderbook:");
    print_orderbook(&merged_orderbook);

    // Convert the merged orderbook to a summary
    let summary = orderbook_to_summary(&merged_orderbook);

    // Send the summary through the channel
    if let Err(err) = sender.send(Ok(summary)).await {
        return Err(Box::new(err));
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command-line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: cargo run -- <symbol> [depth]");
        return Ok(());
    }
    let symbol = args[1].clone();
    let depth = args.get(2).and_then(|d| d.parse().ok()).unwrap_or(10);

    let addr = "0.0.0.0:50051".parse()?;
    let orderbook_aggregator = OrderbookAggregatorService { symbol, depth };

    println!("gRPC server listening on {}", addr);

    Server::builder()
        .add_service(OrderbookAggregatorServer::new(orderbook_aggregator))
        .serve(addr)
        .await?;

    Ok(())
}
