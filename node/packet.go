/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

package node

import "hyperchain/blockchain"

type PacketKind int
const (
    PacketHandShake = PacketKind(iota)
    PacketPing
    PacketBlock
)

type Packet struct {
    Kind PacketKind
    Block blockchain.Block
}

