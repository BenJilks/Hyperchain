/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use crate::block::Block;
use crate::block::validate::BlockValidationResult;

pub fn mine_block(mut block: Block) -> Block
{
    while block.validate_pow().unwrap() != BlockValidationResult::Ok {
        block.header.pow += 1;
    }

    block
}

