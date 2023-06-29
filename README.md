# OrderBook


### Overview
- The gRPC server connects to [Binance](https://github.com/binance/binance-spot-api-docs/blob/master/web-socket-streams.md#partial-book-depth-streams) and [Bitstamp](https://www.bitstamp.net/websocket/v2/) websocket servers.
- Given a symbol and depth (optional, default value 10, can be 5, 10, or 20), the gRPC server pulls orderbooks from both the exchanges.
- Returns a merged orderbook, with top `depth` bids, asks and the spread (top bid - top ask). If there are two bids/asks with same price, one with more volume is placed higher than the lower volume in orderbook.
- The client connects with the server to read the ordebrook in a stream as it is returned by the server.
- Note: Binance exchange can return the orderbooks in different update speeds i.e. 1000ms(default) and 100ms. In the code I'm using 100ms, but the url can be changed.

### Approach
- **server**: sets up a gRPC server that aggregates order book data from Binance and Bitstamp exchanges, processes the data in real-time, and provides a streaming API to clients for accessing the summarized order book data. 
   - `process_socket_messages` function processes messages received from the WebSocket connections to Binance and Bitstamp exchanges.  
   It updates the order books whenever a new message is recieved from either of the websockets and sends the updated summary to a sender.

   - `OrderbookAggregatorService` struct represents the implementation of the gRPC service for the order book aggregator. It holds the depth parameter and WebSocket connections to Binance and Bitstamp.

   - `OrderbookAggregator` trait implementation for `OrderbookAggregatorService` defines the book_summary method, which is the gRPC endpoint for streaming order book summaries (which is localhost::50051 here). It sets up a channel and spawns a task to process socket messages and send summaries to the channel. It returns a stream of order book summaries as the response.

   - The `main` function is the entry point of the program. It parses command-line arguments, establishes WebSocket connections to Binance and Bitstamp, and starts the gRPC server. The order book aggregator service is instantiated with the provided parameters, and the server listens on a specified address.  
&nbsp;
- **client**: sets up a gRPC client that connects to the order book aggregator server and receives a stream of order book summaries. It then prints each received order book summary to the console. 
   - The client sends the request to the server using the book_summary method and receives a stream of order book summaries.

   - The client iterates over the received stream using a while let loop. It prints each received order book summary using the `print_summary` function.  
&nbsp;
  
- **orderbook_helper**: provide functionality to interact with WebSocket connections, process and merge order books, and visualize the order book data  
  - `PriceAmountLevel` struct: Represents a price and amount level for a particular exchange.

  - `OrderBook` struct: Represents the order book for a cryptocurrency symbol, containing bid and ask levels along with the spread.

  - `print_orderbook`: Prints the order book in a formatted manner, displaying the spread, bid and ask levels, and exchange information.

  - `sort_and_trim_levels`: Sorts the price and amount levels in ascending or descending order based on the provided parameters, and returns a trimmed selection of levels up to the specified depth.

  - `process_message`: Processes a message in JSON format received from a cryptocurrency exchange. It extracts the bid and ask levels, calculates the spread, sorts and trims the levels, and returns an OrderBook instance.

  - `merge_orderbooks`: Merges two order books from different exchanges (Binance and Bitstamp) into a single order book. It combines the bid and ask levels, sorts and trims them, and calculates the spread.

  - `binance_connect`: Establishes a WebSocket connection with the Binance exchange. It sends a subscription message to receive real-time updates for the specified symbol and depth, and returns a WebSocket instance for further interaction.

  - `bitstamp_connect`: Establishes a WebSocket connection with the Bitstamp exchange. It sends a subscription message to receive real-time updates for the order book of the specified symbol, and returns a WebSocket instance.


### How to run ?
- For running server, run `./run_server.sh <symbol> [depth, 10 by default]`

- Once the server is connected to the websocket, and starts accepting request, for running client `cargo run --bin orderbook-client`

### What better can be done?
- While maintaining the orderbook, we can explore more data structures, as per the usecase - priority queue for keeping the bids and asks sorted as soon as we add more values to it,  
  a list/vector for price, amount with a mapping from price of bid/ask to its position index in orderbook.

- Just use Summary and Levels provided by protobuf, and don't define OrderBook, PriceAmountLevel in helper function - we can do this, but for easier operations I'm using Summary and Level.

- Use benchmarking and low level optimizations as per the usecase.
