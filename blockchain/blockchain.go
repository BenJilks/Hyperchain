/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

package blockchain

import (
	"time"
    . "hyperchain/blockchain/transaction"
)

type AddError int
const (
    AddErrorInvalidId = AddError(iota)
    AddErrorInvalidPrevBlockHash
    AddErrorInvalidPOW
    AddErrorInvalidTimestamp
)

func (err AddError) Error() string {
    switch err {
    case AddErrorInvalidId:
        return "Block ID is not valid"
    case AddErrorInvalidPrevBlockHash:
        return "Hash of previous block is incorrect"
    case AddErrorInvalidPOW:
        return "Proof of work is invalid"
    case AddErrorInvalidTimestamp:
        return "Block was created before the previous one"
    default:
        panic("Unreachable")
    }
}

type BlockChain struct {
    blocks []Block
}

func NewBlockChain() BlockChain {
    return BlockChain {
        blocks: make([]Block, 0),
    }
}

func (chain *BlockChain) Top() *Block {
    if len(chain.blocks) == 0 {
        return nil
    }

    top := chain.blocks[len(chain.blocks)-1]
    return &top
}

func (chain *BlockChain) ValidateBlock(block Block) error {
    if block.Id > 0 {
        lastBlock := chain.blocks[block.Id - 1]

        if block.Timestamp < lastBlock.Timestamp {
            return AddErrorInvalidTimestamp
        }

        if block.PrevBlock != lastBlock.Hash() {
            return AddErrorInvalidPrevBlockHash
        }
    }

    // TODO: Validate target is correct.

    if !IsValidHashForTarget(block.Hash(), block.Target) {
        return AddErrorInvalidPOW
    }

    for _, transaction := range block.Transactions {
        if err := transaction.Validate(); err != nil {
            return err
        }
    }

    for _, address := range AddressesUsed(block.Transactions) {
        if _, err := chain.WalletStatus(address); err != nil {
            return err
        }
    }

    return nil
}

func (chain *BlockChain) Add(block Block) error {
    if top := chain.Top(); top != nil {
        if block.Id != top.Id + 1 {
            return AddErrorInvalidId
        }
    } else {
        if block.Id != 0 {
            return AddErrorInvalidId
        }
    }

    if err := chain.ValidateBlock(block); err != nil {
        return err
    }

    chain.blocks = append(chain.blocks, block)
    return nil
}

func (chain *BlockChain) sample() (*Block, *Block) {
    if len(chain.blocks) <= BlockSampleSize {
        return nil, nil
    }

    start := chain.blocks[len(chain.blocks)-BlockSampleSize-1]
    end := chain.blocks[len(chain.blocks)-1]
    return &start, &end
}

func (chain *BlockChain) NewBlock(rewardTo [32]byte) Block {
    topId := uint64(0)
    topHash := [32]byte{}
    if top := chain.Top(); top != nil {
        topId = top.Id + 1
        topHash = top.Hash()
    }

    sampleStart, sampleEnd := chain.sample()
    block := Block {
        Id: topId,
        PrevBlock: topHash,
        Timestamp: uint64(time.Now().Unix()),
        Target: CalculateTarget(sampleStart, sampleEnd),
        RewardTo: rewardTo,
        Pow: 0,
    }

    return block
}

func (chain *BlockChain) WalletStatus(address [32]byte) (WalletStatus, error) {
    var status WalletStatus
    for _, block := range chain.blocks {
        if block.RewardTo == address {
            status.Balance += BlockReward
        }

        for _, transaction := range block.Transactions {
            if err := transaction.Apply(&status, address, block.RewardTo); err != nil {
                return WalletStatus{}, err
            }
        }

        if status.Balance < 0 {
            return WalletStatus{}, WalletStatusNegativeBalance
        }
    }

    return status, nil
}

