#!/bin/bash

set -e

tsc typescript/*.ts --out generated/main.js --target ES6
cargo run
