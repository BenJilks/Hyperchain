/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

package wallet

import (
	"encoding/binary"
    "math"
)

func IntToBytes(i int) []byte {
    bytes := make([]byte, 4)
    binary.LittleEndian.PutUint32(bytes, uint32(i))
    return bytes
}

func Uint64AsBytes(i uint64) []byte {
    bytes := make([]byte, 8)
    binary.LittleEndian.PutUint64(bytes, i)
    return bytes
}

func Float32AsBytes(f float32) []byte {
    bytes := make([]byte, 4)
    floatBits := math.Float32bits(f)
    binary.LittleEndian.PutUint32(bytes, floatBits)
    return bytes
}

