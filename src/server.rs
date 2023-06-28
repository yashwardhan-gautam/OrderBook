mod orderbook_helper;
use orderbook_helper::{
    binance_connect, bitstamp_connect, merge_orderbooks, print_orderbook, process_message,
    OrderBook,
};

pub mod orderbook {
    tonic::include_proto!("orderbook");
}

use futures::stream::{Stream, StreamExt};
use orderbook::orderbook_aggregator_server::{OrderbookAggregator, OrderbookAggregatorServer};
use orderbook::{Empty, Level, Summary};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tokio::select;
use tokio::sync::mpsc;
use tokio::task::spawn_blocking;
use tonic::{transport::Server, Code, Request, Response, Status};
use tungstenite::client::AutoStream;
use tungstenite::WebSocket;

#[derive(Default, Clone)]

// depth is required to trim the messages from websocket
// sockets are required so we don't have to connect everytime
struct OrderbookAggregatorService {
    depth: u32,
    binance_socket: Option<Arc<Mutex<WebSocket<AutoStream>>>>,
    bitstamp_socket: Option<Arc<Mutex<WebSocket<AutoStream>>>>,
}

#[tonic::async_trait]
impl OrderbookAggregator for OrderbookAggregatorService {
    type BookSummaryStream =
        Pin<Box<dyn Stream<Item = Result<Summary, Status>> + Send + Sync + 'static>>;

    async fn book_summary(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::BookSummaryStream>, Status> {
        let (sender, receiver) = mpsc::channel(100);
        let depth = self.depth;
        let binance_socket = self.binance_socket.clone().map(|s| Arc::clone(&s));
        let bitstamp_socket = self.bitstamp_socket.clone().map(|s| Arc::clone(&s));

        let summary_sender = sender.clone();
        let binance_socket_clone = binance_socket.clone();
        let bitstamp_socket_clone = bitstamp_socket.clone();

        tokio::spawn(async move {
            let subscription_result = process_socket_messages(
                summary_sender,
                depth,
                binance_socket_clone,
                bitstamp_socket_clone,
            )
            .await;

            if let Err(err) = subscription_result {
                eprintln!("Error during subscription: {}", err);
            }
        });

        let stream = tokio_stream::wrappers::ReceiverStream::new(receiver).map(
            |result: Result<Summary, ()>| {
                result.map_err(|_| Status::new(Code::Internal, "Unknown error occurred"))
            },
        );

        let response_stream: Self::BookSummaryStream = Box::pin(stream);
        Ok(Response::new(response_stream))
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

async fn process_socket_messages(
    sender: tokio::sync::mpsc::Sender<Result<Summary, ()>>,
    depth: u32,
    binance_socket: Option<Arc<Mutex<WebSocket<AutoStream>>>>,
    bitstamp_socket: Option<Arc<Mutex<WebSocket<AutoStream>>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let binance_orderbook = Arc::new(Mutex::new(OrderBook::new()));
    let bitstamp_orderbook = Arc::new(Mutex::new(OrderBook::new()));

    while let (Some(binance_socket), Some(bitstamp_socket)) =
        (binance_socket.clone(), bitstamp_socket.clone())
    {
        select! {
            binance_msg = spawn_blocking({
                let binance_socket = binance_socket.clone();
                move || -> Result<tungstenite::Message, tungstenite::Error> {
                    let mut binance_socket = binance_socket.lock().unwrap();
                    binance_socket.read_message().map_err(Into::into)
                }
            }) => {
                if let Ok(message) = binance_msg? {
                    let message_text = message.to_text().unwrap_or("");
                    if let Some(new_orderbook) =
                        process_message(message_text, "binance", depth as usize)
                    {
                        let mut binance_orderbook = binance_orderbook.lock().unwrap();
                        *binance_orderbook = new_orderbook.clone();
                        let merged_orderbook = merge_orderbooks(
                            &new_orderbook,
                            &bitstamp_orderbook.lock().unwrap(),
                            depth as usize,
                        );
                        println!("Orderbook updated by Binance:");
                        print_orderbook(&merged_orderbook);
                        let summary = orderbook_to_summary(&merged_orderbook);
                        sender.try_send(Ok(summary)).unwrap();
                    }
                }
            }
            bitstamp_msg = spawn_blocking({
                let bitstamp_socket = bitstamp_socket.clone();
                move || -> Result<tungstenite::Message, tungstenite::Error> {
                    let mut bitstamp_socket = bitstamp_socket.lock().unwrap();
                    bitstamp_socket.read_message().map_err(Into::into)
                }
            }) => {
                if let Ok(message) = bitstamp_msg? {
                    let message_text = message.to_text().unwrap_or("");
                    if let Some(new_orderbook) =
                        process_message(message_text, "bitstamp", depth as usize)
                    {
                        let mut bitstamp_orderbook = bitstamp_orderbook.lock().unwrap();
                        *bitstamp_orderbook = new_orderbook.clone();
                        let merged_orderbook = merge_orderbooks(
                            &binance_orderbook.lock().unwrap(),
                            &new_orderbook,
                            depth as usize,
                        );
                        println!("Orderbook updated by Bitstamp:");
                        print_orderbook(&merged_orderbook);
                        let summary = orderbook_to_summary(&merged_orderbook);
                        sender.try_send(Ok(summary)).unwrap();
                    }
                }
            }
        }
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

    let binance_socket = binance_connect(&symbol, depth).await?;
    let bitstamp_socket = bitstamp_connect(&symbol).await?;

    println!("gRPC server listening on {}", addr);
    let orderbook_aggregator = OrderbookAggregatorService {
        depth,
        binance_socket: Some(Arc::new(Mutex::new(binance_socket))),
        bitstamp_socket: Some(Arc::new(Mutex::new(bitstamp_socket))),
    };

    Server::builder()
        .add_service(OrderbookAggregatorServer::new(orderbook_aggregator))
        .serve(addr)
        .await?;

    Ok(())
}
