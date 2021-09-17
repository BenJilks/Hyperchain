#!/usr/bin/env bash

pushd lib
cargo test --no-run && \
    kcov --include-path src/ ../cov `ls -lt target/debug/deps/libhyperchain-???????????????? | 
        head -n 1 | 
        awk '{print $9}'`
popd

pushd service
cargo test --no-run && \
    kcov --include-path src/ ../cov `ls -lt target/debug/deps/hyperchain_service-???????????????? | 
        head -n 1 | 
        awk '{print $9}'`
popd

