/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

package node

import (
	. "hyperchain/blockchain"
	"fmt"
	"time"
)

const MinerCheckInterval = 100
const MinerDelayTimeMS = 10

func tryMine(block *Block) bool {
    for i := 0; i < MinerCheckInterval; i++ {
        if IsValidHashForTarget(block.Hash(), block.Target) {
            return true
        }

        block.Pow += 1
        time.Sleep(time.Millisecond * MinerDelayTimeMS)
    }

    return false
}

func startMiner(blocks chan Block) {
    fmt.Println("Started miner")

    var currBlock Block
    hasBlock := false

    for {
        didStartNewBlock := false

        select {
        case currBlock = <- blocks:
            didStartNewBlock = true
            hasBlock = true
        default:
            if !hasBlock {
                currBlock = <- blocks
                didStartNewBlock = true
                hasBlock = true
            }
        }

        if didStartNewBlock {
            fmt.Printf("Started mining block %d\n", currBlock.Id)
        }
        
        if tryMine(&currBlock) {
            fmt.Printf("Successfully mined block %d!\n", currBlock.Id)
            blocks <- currBlock
            hasBlock = false
        }
    }
}

func Miner() chan Block {
    blocks := make(chan Block)
    go startMiner(blocks)

    return blocks
}

