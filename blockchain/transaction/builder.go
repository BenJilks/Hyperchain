/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

package transaction

import (
    . "hyperchain/blockchain/wallet"
)

type TransactionBuilder struct {
    transaction Transaction
    wallets []Wallet
}

func NewTransactionBuilder(id uint64, fee float32) TransactionBuilder {
    return TransactionBuilder {
        transaction: Transaction {
            Id: id,
            Fee: fee,
        },
        wallets: []Wallet{},
    }
}

func (builder TransactionBuilder) AddInput(wallet Wallet, amount float32) TransactionBuilder {
    builder.transaction.Inputs = append(builder.transaction.Inputs, Input {
        KeyN: *wallet.Key.N,
        KeyE: wallet.Key.E,
        Amount: amount,
    })

    builder.wallets = append(builder.wallets, wallet)
    return builder
}

func (builder TransactionBuilder) AddOutput(output Output) TransactionBuilder {
    builder.transaction.Outputs = append(builder.transaction.Outputs, output)
    return builder
}

func (builder TransactionBuilder) Build() (Transaction, error) {
    for i := range builder.transaction.Inputs {
        input := &builder.transaction.Inputs[i]
        wallet := builder.wallets[i]
        data := builder.transaction.Hash()

        signature, err := wallet.Sign(data[:])
        if err != nil {
            return Transaction{}, err
        }

        input.Signature = signature
    }

    return builder.transaction, nil
}

