#!/bin/bash

echo Running rust program...
cargo run > out.ll
echo Compiling...
llc -o out.o out.ll -filetype=obj -O3
echo Linking...
ld -o hello -dynamic-linker /lib64/ld-linux-x86-64.so.2 out.o ./helper/flush_stdout.o -lc
rm out.ll out.o



