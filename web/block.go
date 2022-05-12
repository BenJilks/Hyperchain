/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

package main

import (
	"net/http"
	"strconv"
    "hyperchain/node"
    . "hyperchain/blockchain"
    . "hyperchain/blockchain/transaction"
)

type InputData struct {
    Address string
    Amount float32
}

type TransactionData struct {
    ID string
    TotalAmount float32
    Fee float32
    
    IsPending bool
    Block uint64

    ContainsChunks bool
    DataSize uint64
    ChunkCount uint64
    Data []struct {
        Hash string
    }

    Inputs []InputData
    Outputs []InputData
}

type BlockData struct {
    ID uint64
    
    LastBlockID uint64
    NextBlockID uint64
    Top uint64

    Timestamp uint64
    Winner string
    MerkleRoot string
    Difficulty float64
    POW uint64

    Transactions []TransactionData
}

func block(w http.ResponseWriter, request *http.Request) {
    id, err := strconv.ParseInt(request.URL.Query().Get("id"), 10, 32)
    if err != nil {
        http.Redirect(w, request, "/", 301)
        return
    }

    response, errMsg := node.SendIpc(node.Command {
        Kind: node.CommandBlock,
        ID: int(id),
    })

    if errMsg != "" {
        http.Error(w, errMsg, 500)
        return
    }

    stats, errMsg := node.SendIpc(node.Command {
        Kind: node.CommandStats,
    })

    if errMsg != "" {
        http.Error(w, errMsg, 500)
        return
    }

    block := response.Block
    top := stats.Block
    
    transactions := make([]TransactionData, 0)
    for _, transaction := range block.Transactions {
        inputs := make([]InputData, 0)
        totalAmount := float32(0.0)
        for _, input := range transaction.Inputs {
            inputs = append(inputs, InputData {
                Address: input.Address().ToString(),
                Amount: input.Amount,
            })
            totalAmount += input.Amount
        }

        outputs := make([]InputData, 0)
        for _, output := range transaction.Outputs {
            outputs = append(outputs, InputData {
                Address: output.Interface().Addresses()[0].ToString(),
                Amount: output.Interface().Cost(),
            })
        }

        transactions = append(transactions, TransactionData {
            ID: transaction.Hash().ToString(),
            TotalAmount: totalAmount,
            Fee: transaction.Fee,
            
            IsPending: false,
            Block: block.Id,

            ContainsChunks: false,
            DataSize: 0,
            ChunkCount: 0,
            Data: make([]struct{Hash string}, 0),

            Inputs: inputs,
            Outputs: outputs,
        })
    }

    templates.ExecuteTemplate(w, "Block", BlockData {
        ID: block.Id,

        LastBlockID: block.Id - 1,
        NextBlockID: block.Id + 1,
        Top: top.Id,

        Timestamp: block.Timestamp,
        Winner: block.RewardTo.ToString(),
        MerkleRoot: MerkleRoot(block.Transactions).ToString(),
        Difficulty: Difficulty(block.Target),
        POW: block.Pow,

        Transactions: transactions,
    })
}

