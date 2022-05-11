/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

package transaction

import (
	"crypto"
	"crypto/rsa"
	"crypto/sha256"
	"math/big"
    "hash"
)

type Input struct {
    KeyN big.Int
    KeyE int
    Signature []byte
    Amount float32
}

func (input *Input) Address() [32]byte {
    var address [32]byte
    hasher := sha256.New()
    hasher.Write(input.KeyN.Bytes())
    hasher.Write(IntToBytes(input.KeyE))
    copy(address[:], hasher.Sum(nil))

    return address
}

type Output interface {
    hash(hash.Hash)
    cost() float32
    addresses() [][32]byte
    apply(*WalletStatus, [32]byte) (bool, error)
}

type Transaction struct {
    Id uint64
    Fee float32
    Inputs []Input
    Outputs []Output
}

func (transaction *Transaction) Hash() [32]byte {
    hasher := sha256.New()
    hasher.Write(Uint64AsBytes(transaction.Id))
    hasher.Write(Float32AsBytes(transaction.Fee))
    for _, input := range transaction.Inputs {
        hasher.Write(input.KeyN.Bytes())
        hasher.Write(IntToBytes(input.KeyE))
        hasher.Write(Float32AsBytes(input.Amount))
    }
    for _, output := range transaction.Outputs {
        output.hash(hasher)
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
        outputAmount += output.cost()
    }

    // FIXME: Comparing float values like this is a no-no
    if inputAmount != outputAmount {
        return TransactionInsufficientInput
    }

    return nil
}

func (transaction *Transaction) Apply(status *WalletStatus, address [32]byte, rewardTo [32]byte) error {
    areWeInvolved := false

    for _, input := range transaction.Inputs {
        if input.Address() == address {
            status.Balance -= input.Amount
            areWeInvolved = true
        }
    }

    for _, output := range transaction.Outputs {
        involved, err := output.apply(status, address)
        areWeInvolved = areWeInvolved || involved

        if err != nil {
            return err
        }
    }

    if rewardTo == address {
        status.Balance += transaction.Fee
        areWeInvolved = true
    }

    if areWeInvolved {
        if transaction.Id <= status.LastId {
            return WalletStatusInvalidId
        }

        status.LastId = transaction.Id
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
            for _, address := range output.addresses() {
                if !contains(addresses, address) {
                    addresses = append(addresses, address)
                }
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

