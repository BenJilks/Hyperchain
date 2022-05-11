/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

package transaction

import (
	"crypto"
	"crypto/rand"
	"crypto/rsa"
	"crypto/sha256"
	"encoding/json"
	"io/ioutil"
	"os"
)

type Wallet struct {
    Key *rsa.PrivateKey
}

type WalletStatus struct {
    Balance float32
    LastId uint64
}

type WalletStatusError int
const (
    WalletStatusNegativeBalance = WalletStatusError(iota)
    WalletStatusInvalidId
)

func (err WalletStatusError) Error() string {
    switch err {
    case WalletStatusNegativeBalance:
        return "Transaction results in a negative balance"
    case WalletStatusInvalidId:
        return "Transaction ID is non-sequential"
    default:
        panic(err)
    }
}

func NewWallet() (Wallet, error) {
    privateKey, err := rsa.GenerateKey(rand.Reader, 2048)
    if err != nil {
        return Wallet{}, err
    }

    return Wallet {
        Key: privateKey,
    }, nil
}

func LoadWallet(filePath string) (Wallet, error) {
    file, err := os.Open(filePath)
    if err != nil {
        return Wallet{}, err
    }
    
    bytes, err := ioutil.ReadAll(file)
    if err != nil {
        return Wallet{}, err
    }

    var key rsa.PrivateKey
    if err := json.Unmarshal(bytes, &key); err != nil {
        return Wallet{}, err
    }

    return Wallet {
        Key: &key,
    }, nil
}

func (wallet *Wallet) Save(filePath string) error {
    file, err := os.Create(filePath)
    if err != nil {
        return err
    }

    json_data, err := json.Marshal(wallet.Key)
    if err != nil {
        return err
    }

    n, err := file.Write(json_data)
    if err != nil {
        return err
    }

    if n != len(json_data) {
        panic("Failed to write wallet")
    }
    return nil
}

func (wallet *Wallet) Address() [32]byte {
    publicKey := wallet.Key.PublicKey

    hasher := sha256.New()
    hasher.Write(publicKey.N.Bytes())
    hasher.Write(IntToBytes(publicKey.E))
    
    var address [32]byte
    copy(address[:], hasher.Sum(nil))
    return address
}

func (wallet *Wallet) Sign(transaction Transaction) ([]byte, error) {
    hash := transaction.Hash()
    signature, err := rsa.SignPKCS1v15(rand.Reader, wallet.Key, crypto.SHA256, hash[:])
    if err != nil {
        return []byte{}, err
    }

    return signature, nil
}

