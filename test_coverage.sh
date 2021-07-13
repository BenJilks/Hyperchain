#!/usr/bin/env bash

cargo test --no-run && \
	kcov --include-path src/ target/cov `ls -lt target/debug/deps/decentralized_web-???????????????? | head -n 1 | awk '{print $9}'` test_block_v

