#!/usr/bin/env bash

cargo test --no-run && \
	kcov --include-path src/ target/cov target/debug/deps/decentralized_web-59ce8da70db1b0eb

