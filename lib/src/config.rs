/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */


pub const BLOCK_TIME: u64 = 10 * 1000; // 10 second blocks
// pub const BLOCK_TIME: u64 = 10 * 60 * 1000; // 10 minute blocks
pub const BLOCK_SAMPLE_SIZE: u64 = 100;
pub const PAGE_CHUNK_SIZE: usize = 1000 * 1000; // 1MB

pub const PUB_KEY_LEN: usize = 256;
pub const HASH_LEN: usize = 32;

