/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

package main

import (
	"fmt"
	"hyperchain/blockchain"
	"hyperchain/node"
)

func main() {
    fmt.Println("Staring Node")

    wallet, err := blockchain.NewWallet()
    if err != nil {
        panic(err)
    }

    node.StartNode(wallet.Address(), 8080)
}

