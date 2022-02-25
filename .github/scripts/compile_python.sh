#!/bin/bash
pwd
ls
rustup update stable && rustup default stable
cargo build --release
git clone https://github.com/python/cpython.git --depth 1
export "CC=???"
