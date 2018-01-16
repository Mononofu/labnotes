#!/bin/bash

set -e

echo
echo
echo
echo
echo
tsc typescript/*.ts --out generated/main.js --target ES6
cargo run
