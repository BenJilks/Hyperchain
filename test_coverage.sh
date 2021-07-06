#!/usr/bin/env bash

cargo test --no-run && \
	kcov --include-path src/ target/cov `find target/debug/deps -name "decentralized_web-????????????????" -printf '%T+ %p\n' | sort -r | head | head -n 1 | awk '{print $2}'` 

