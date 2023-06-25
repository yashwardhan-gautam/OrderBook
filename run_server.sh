#!/bin/bash

symbol=$1
depth=${2:-10}

cargo run --bin orderbook-server -- $symbol $depth
