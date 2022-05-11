/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

package blockchain

import (
    "testing"
    . "hyperchain/blockchain/transaction"
)

func (chain *BlockChain) testBlock(t *testing.T,
                                   expect error,
                                   configBlock func(block *Block)) Block {
    block := chain.NewBlock([32]byte{})
    configBlock(&block)
    block.Mine()
    if err := chain.Add(block); err != expect {
        t.Error(err)
    }

    return block
}

func TestBlockChainAddId(t *testing.T) {
    chain := NewBlockChain()

    chain.testBlock(t, AddErrorInvalidId, func(block *Block) {
        block.Id = 1
    })
    chain.testBlock(t, nil, func(block *Block) {
        block.Id = 0
    })
    chain.testBlock(t, AddErrorInvalidId, func(block *Block) {
        block.Id = 0
    })
    chain.testBlock(t, nil, func(block *Block) {
        block.Id = 1
    })
}

func TestBlockChainAddTimestamp(t *testing.T) {
    chain := NewBlockChain()

    chain.testBlock(t, nil, func(block *Block) {
        block.Timestamp = 0
    })
    chain.testBlock(t, nil, func(block *Block) {
        block.Timestamp = 10
    })
    chain.testBlock(t, AddErrorInvalidTimestamp, func(block *Block) {
        block.Timestamp = 4
    })
}

func TestBlockChainAddPrevBlock(t *testing.T) {
    chain := NewBlockChain()

    block_a := chain.testBlock(t, nil, func(block *Block) {})
    chain.testBlock(t, nil, func(block *Block) {
        block.PrevBlock = block_a.Hash()
    })
    chain.testBlock(t, AddErrorInvalidPrevBlockHash, func(block *Block) {
        block.PrevBlock = [32]byte{}
    })
}

func TestBlockChainAddPOW(t *testing.T) {
    chain := NewBlockChain()
    chain.testBlock(t, nil, func(block *Block) {})

    block := chain.NewBlock([32]byte{})
    if err := chain.Add(block); err != AddErrorInvalidPOW {
        t.Error(err)
    }
}

func TestBlockChainAddTransaction(t *testing.T) {
    chain := NewBlockChain()

    wallet_a, err := NewWallet()
    if err != nil {
        t.Error(err)
    }

    wallet_b, err := NewWallet()
    if err != nil {
        t.Error(err)
    }

    block := chain.NewBlock(wallet_a.Address())
    testPageCost := float32(1010.0/(1000.0*1000.0))
    transaction, err := NewTransactionBuilder(1, 1).
        AddInput(wallet_a, 11).
        AddOutput(&Transfer {
            Address: wallet_b.Address(),
            Amount: 10.0 - testPageCost,
        }).
        AddOutput(&NewPage {
            Address: wallet_b.Address(),
            Name: "index.html",
            Length: 1000,
            Chunks: make([][32]byte, 0),
        }).
        Build()
    if err != nil {
        t.Error(err)
    }

    block.Transactions = append(block.Transactions, transaction)
    block.Mine()
    if err := chain.Add(block); err != nil {
        t.Error(err)
    }

    status_a, err := chain.WalletStatus(wallet_a.Address())
    if err != nil {
        t.Error(err)
    }

    status_b, err := chain.WalletStatus(wallet_b.Address())
    if err != nil {
        t.Error(err)
    }

    if status_a.Balance != BlockReward - 10 && status_b.Balance != 10.0 - testPageCost {
        t.Errorf("Expected balance a = %f and b = %f. Got %f and %f instead",
            BlockReward - 10, 10.0 - testPageCost, status_a.Balance, status_b.Balance)
    }

    if status_a.LastId != 1 && status_b.LastId != 1 {
        t.Errorf("Expected last id a = 1 and b = 1. Got %d and %d instead",
            status_a.LastId, status_b.LastId)
    }
}

// TODO: Test invalid transaction cases

