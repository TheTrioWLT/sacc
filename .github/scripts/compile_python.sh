#!/bin/bash
rustup update stable && rustup default stable
cargo build --release
git clone https://github.com/python/cpython.git --depth 1
export CC=$(pwd)/target/debug/sacc
echo $CC
cd cpython
if ! ./configure --disable-silent-rules ; then
    echo "/n/n/n==============================================/n/n/n"
    echo "Configure failed. Log below:"
    cat config.log
    exit 1
fi
echo "Configure succeeded!"
make -j8
