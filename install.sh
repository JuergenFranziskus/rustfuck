#!/bin/bash

cargo build --release
sudo install ./target/release/rustfuck /usr/local/bin


