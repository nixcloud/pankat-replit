#!/bin/bash
wasm-pack build --release --target web --no-typescript --out-name js-framework-benchmark-sauron --out-dir bundled-dist && \
 basic-http-server -a 0.0.0.0:8888
