#!/bin/sh
rm ./bin/*.so
rm ./bin/*.dll

cp ./target/x86_64-unknown-linux-gnu/release/libemojikanban.so ./bin
cp ./target/x86_64-pc-windows-msvc/release/emojikanban.dll ./bin
