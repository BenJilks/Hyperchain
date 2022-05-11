/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

package transaction

import (
    "hash"
)

type NewPage struct {
    Address [32]byte
    Name string
    Length int
    Chunks [][32]byte
}

func (page *NewPage) hash(hasher hash.Hash) {
    hasher.Write(page.Address[:])
    hasher.Write([]byte(page.Name))
    hasher.Write(IntToBytes(page.Length))
    for _, chunk := range page.Chunks {
        hasher.Write(chunk[:])
    }
}

func (page *NewPage) cost() float32 {
    length := page.Length + len(page.Name)
    return float32(length) / (1000.0*1000.0)
}

func (page *NewPage) addresses() [][32]byte {
    return [][32]byte { page.Address }
}

func (page *NewPage) apply(status *WalletStatus, address [32]byte) (bool, error) {
    if page.Address == address {
        return true, nil
    } else {
        return false, nil
    }
}

