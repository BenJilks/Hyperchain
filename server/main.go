/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

package main

import (
	"flag"
	"fmt"
	. "hyperchain/blockchain/wallet"
	"hyperchain/node"
)

func main() {
    walletPath := flag.String("wallet", "", "Wallet file path")
    flag.Parse()

    var wallet Wallet
    var err error

    if *walletPath != "" {
        fmt.Printf("Using wallet '%s'\n", *walletPath)
        wallet, err = LoadWallet(*walletPath)
    } else {
        wallet, err = NewWallet()
    }

    if err != nil {
        panic(err)
    }

    fmt.Println("Staring Node")
    node.StartNode(wallet.Address(), 8080)
}

