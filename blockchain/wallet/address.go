/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

package wallet

import "encoding/base32"

type Address [32]byte

func DecodeAddress(str string) (Address, error) {
    bytes, err := base32.StdEncoding.DecodeString(str)
    if err != nil {
        return Address{}, err
    }

    var address [32]byte
    copy(address[:], bytes)
    return address, nil
}

func (address Address) ToString() string {
    return base32.StdEncoding.EncodeToString(address[:])
}

