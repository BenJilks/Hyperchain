/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

package transaction

import (
    "hash"
)

type Transfer struct {
    Address [32]byte
    Amount float32
}

func (transfer *Transfer) hash(hasher hash.Hash) {
    hasher.Write(transfer.Address[:])
    hasher.Write(Float32AsBytes(transfer.Amount))
}

func (transfer *Transfer) cost() float32 {
    return transfer.Amount
}

func (transfer *Transfer) addresses() [][32]byte {
    return [][32]byte { transfer.Address }
}

func (transfer *Transfer) apply(status *WalletStatus, address [32]byte) (bool, error) {
    involved := false

    if transfer.Address == address {
        status.Balance += transfer.Amount
        involved = true
    }

    return involved, nil
}

