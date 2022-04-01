/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

package blockchain

import (
	"crypto"
	"crypto/rsa"
	"crypto/sha256"
	"encoding/binary"
	"math/big"
)

type Input struct {
    KeyN big.Int
    KeyE int
    Signature []byte
    Amount float32
}

func intToBytes(i int) []byte {
    bytes := make([]byte, 4)
    binary.LittleEndian.PutUint32(bytes, uint32(i))
    return bytes
}

func (input *Input) Address() [32]byte {
    var address [32]byte
    hasher := sha256.New()
    hasher.Write(input.KeyN.Bytes())
    hasher.Write(intToBytes(input.KeyE))
    copy(address[:], hasher.Sum(nil))

    return address
}

type Output struct {
    Address [32]byte
    Amount float32
}

type Transaction struct {
    Id uint64
    Fee float32
    Inputs []Input
    Outputs []Output
}

func (transaction *Transaction) Hash() [32]byte {
    hasher := sha256.New()
    hasher.Write(uint64AsBytes(transaction.Id))
    hasher.Write(float32AsBytes(transaction.Fee))
    for _, input := range transaction.Inputs {
        hasher.Write(input.KeyN.Bytes())
        hasher.Write(intToBytes(input.KeyE))
        hasher.Write(float32AsBytes(input.Amount))
    }
    for _, output := range transaction.Outputs {
        hasher.Write(output.Address[:])
        hasher.Write(float32AsBytes(output.Amount))
    }

    var hash [32]byte
    copy(hash[:], hasher.Sum(nil))
    return hash
}

type TransactionError int
const (
    TransactionInsufficientInput = TransactionError(iota)
)

func (err TransactionError) Error() string {
    switch err {
    case TransactionInsufficientInput:
        return "Insufficient input for output"
    default:
        panic(err)
    }
}

func (transaction *Transaction) Validate() error {
    hash := transaction.Hash()

    inputAmount := float32(0)
    for _, input := range transaction.Inputs {
        inputAmount += input.Amount
        publicKey := rsa.PublicKey {
            N: &input.KeyN,
            E: input.KeyE,
        }

        sig := input.Signature
        err := rsa.VerifyPKCS1v15(&publicKey, crypto.SHA256, hash[:], sig[:])
        if err != nil {
            return err
        }
    }

    outputAmount := transaction.Fee
    for _, output := range transaction.Outputs {
        outputAmount += output.Amount
    }

    // FIXME: Comparing float values like this is a no-no
    if inputAmount != outputAmount {
        return TransactionInsufficientInput
    }

    return nil
}

func contains(list [][32]byte, item [32]byte) bool {
    for _, it := range list {
        if it == item {
            return true
        }
    }

    return false
}

func AddressesUsed(transactions []Transaction) [][32]byte {
    var addresses [][32]byte
    for _, transaction := range transactions {
        for _, input := range transaction.Inputs {
            address := input.Address()
            if !contains(addresses, address) {
                addresses = append(addresses, address)
            }
        }

        for _, output := range transaction.Outputs {
            if !contains(addresses, output.Address) {
                addresses = append(addresses, output.Address)
            }
        }
    }

    return addresses
}

func merkleRootForNodes(nodes [][32]byte) [32]byte {
    if len(nodes) == 0 {
        return [32]byte{}
    }
    if len(nodes) == 1 {
        return nodes[0]
    }

    middle := len(nodes) / 2
    nodeA := merkleRootForNodes(nodes[:middle])
    nodeB := merkleRootForNodes(nodes[middle:])

    hasher := sha256.New()
    hasher.Write(nodeA[:])
    hasher.Write(nodeB[:])

    var result [32]byte
    copy(result[:], hasher.Sum(nil))
    return result
}

func MerkleRoot(transactions []Transaction) [32]byte {
    nodes := make([][32]byte, len(transactions))
    for i, transaction := range transactions {
        nodes[i] = transaction.Hash()
    }

    return merkleRootForNodes(nodes)
}

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

func (builder TransactionBuilder) AddOutput(address [32]byte, amount float32) TransactionBuilder {
    builder.transaction.Outputs = append(builder.transaction.Outputs, Output {
        Address: address,
        Amount: amount,
    })
    return builder
}

func (builder TransactionBuilder) Build() (Transaction, error) {
    for i := range builder.transaction.Inputs {
        input := &builder.transaction.Inputs[i]
        wallet := builder.wallets[i]

        signature, err := wallet.Sign(builder.transaction)
        if err != nil {
            return Transaction{}, err
        }

        input.Signature = signature
    }

    return builder.transaction, nil
}

