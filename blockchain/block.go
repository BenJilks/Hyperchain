/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

package blockchain

import (
	"crypto/sha256"
	"encoding/binary"
	"math"
)

const BlockReward = float32(100)

type Block struct {
    Id uint64
    PrevBlock [32]byte
    Timestamp uint64
    Target Target
    RewardTo [32]byte
    Transactions []Transaction
    Pow uint64
}

func uint64AsBytes(i uint64) []byte {
    bytes := make([]byte, 8)
    binary.LittleEndian.PutUint64(bytes, i)
    return bytes
}

func float32AsBytes(f float32) []byte {
    bytes := make([]byte, 4)
    floatBits := math.Float32bits(f)
    binary.LittleEndian.PutUint32(bytes, floatBits)
    return bytes
}

func (block *Block) Hash() [32]byte {
    hasher := sha256.New()
    merkleRoot := MerkleRoot(block.Transactions)
    hasher.Write(uint64AsBytes(block.Id))
    hasher.Write(block.PrevBlock[:])
    hasher.Write(uint64AsBytes(block.Timestamp))
    hasher.Write(block.Target[:])
    hasher.Write(block.RewardTo[:])
    hasher.Write(merkleRoot[:])
    hasher.Write(uint64AsBytes(block.Pow))
    
    var hash [32]byte
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

