/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

package transaction

import (
    "hash"
    . "hyperchain/blockchain/wallet"
)

type Transfer struct {
    Address Address
    Amount float32
}

func (transfer *Transfer) hash(hasher hash.Hash) {
    hasher.Write(transfer.Address[:])
    hasher.Write(Float32AsBytes(transfer.Amount))
}

func (transfer *Transfer) Cost() float32 {
    return transfer.Amount
}

func (transfer *Transfer) Addresses() []Address {
    return []Address { transfer.Address }
}

func (transfer *Transfer) apply(status *WalletStatus, address Address) (bool, error) {
    involved := false

    if transfer.Address == address {
        status.Balance += transfer.Amount
        involved = true
    }

    return involved, nil
}

