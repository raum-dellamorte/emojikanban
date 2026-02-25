#!/bin/sh
cargo build -r --target x86_64-unknown-linux-gnu
cargo xwin build -r --target x86_64-pc-windows-msvc
