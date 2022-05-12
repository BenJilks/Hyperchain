/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

package blockchain

import (
	"crypto/sha256"
    . "hyperchain/blockchain/transaction"
    . "hyperchain/blockchain/wallet"
)

const BlockReward = float32(100)

type Block struct {
    Id uint64
    PrevBlock Address
    Timestamp uint64
    Target Target
    RewardTo Address
    Transactions []Transaction
    Pow uint64
}

func (block *Block) Hash() Address {
    hasher := sha256.New()
    merkleRoot := MerkleRoot(block.Transactions)
    hasher.Write(Uint64AsBytes(block.Id))
    hasher.Write(block.PrevBlock[:])
    hasher.Write(Uint64AsBytes(block.Timestamp))
    hasher.Write(block.Target[:])
    hasher.Write(block.RewardTo[:])
    hasher.Write(merkleRoot[:])
    hasher.Write(Uint64AsBytes(block.Pow))
    
    var hash Address
    copy(hash[:], hasher.Sum(nil))
    return hash
}

func (block *Block) Mine() {
    for !IsValidHashForTarget(block.Hash(), block.Target) {
        block.Pow += 1
    }
}

func (block Block) IsNextTo(last Block) bool {
    if block.Id + 1 != last.Id {
        return false
    }

    if block.Timestamp < last.Timestamp {
        return false
    }

    if block.PrevBlock != last.Hash() {
        return false
    }

    return true
}

