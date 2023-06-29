import asyncio
import websockets
import json
import time
import sys


async def monitor_binance_endpoint(symbol, depth=10):
    endpoint = "wss://stream.binance.com:9443/ws"
    binance_url = f"{endpoint}/{symbol.lower()}@depth{depth}@1000ms"

    message_count = 0
    start_time = time.time()

    async with websockets.connect(binance_url) as websocket:  # type: ignore
        while True:
            message = await websocket.recv()

            # Process the received message as needed
            # ...

            message_count += 1
            elapsed_time = time.time() - start_time

            if elapsed_time >= 1.0:
                message_rate = message_count / elapsed_time
                print(f"Binance Message Rate: {message_rate:.2f} messages/sec")

                # Reset the counter and timer
                message_count = 0
                start_time = time.time()


async def monitor_bitstamp_endpoint(symbol):
    bitstamp_url = "wss://ws.bitstamp.net/"
    bitstamp_channel = f"detail_order_book_{symbol}"
    bitstamp_message = json.dumps({
        "event": "bts:subscribe",
        "data": {
            "channel": bitstamp_channel
        }
    })

    message_count = 0
    start_time = time.time()

    async with websockets.connect(bitstamp_url) as websocket:  # type: ignore
        await websocket.send(bitstamp_message)

        while True:
            message = await websocket.recv()

            message_count += 1
            elapsed_time = time.time() - start_time

            if elapsed_time >= 1.0:
                message_rate = message_count / elapsed_time
                print(f"Bitstamp Message Rate: {message_rate:.2f} messages/sec")

                # Reset the counter and timer
                message_count = 0
                start_time = time.time()


# Usage example
if len(sys.argv) < 3:
    print("Usage: python websocket_monitor.py [exchange] [symbol] [depth]")
    sys.exit(1)

exchange = sys.argv[1]
symbol = sys.argv[2]
depth = int(sys.argv[3]) if len(sys.argv) >= 4 else 10

if exchange == "binance":
    loop = asyncio.get_event_loop()
    loop.run_until_complete(monitor_binance_endpoint(symbol, depth))
elif exchange == "bitstamp":
    loop = asyncio.get_event_loop()
    loop.run_until_complete(monitor_bitstamp_endpoint(symbol))
else:
    print("Invalid exchange. Supported exchanges: binance, bitstamp")
