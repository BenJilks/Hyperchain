/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

package main

import (
	"flag"
	"fmt"
	"os"
	"hyperchain/node"
	. "hyperchain/blockchain/transaction"
)

func onError(err string) {
    flag.Usage()
    fmt.Printf("\nError: %s\n", err)
    os.Exit(1)
}

func newWallet(outputPath string) {
    if outputPath == "" {
        onError("No output path given")
    }

    wallet, err := NewWallet()
    if err != nil {
        onError(err.Error())
    }

    fmt.Printf("Creating new wallet '%s'\n", outputPath)
    if err = wallet.Save(outputPath); err != nil {
        onError(err.Error())
    }

    fmt.Print("Done!\n")
}

func connect(connectAddress string) {
    if connectAddress == "" {
        onError("No address given")
    }

    _, err := node.SendIpc(node.Command {
        Kind: node.CommandConnect,
        Address: connectAddress,
    })

    if err != nil {
        onError(err.Error())
    }
}

func balance(walletPath string) {
    wallet, err := LoadWallet(walletPath)
    if err != nil {
        onError(err.Error())
    }

    response, err := node.SendIpc(node.Command {
        Kind: node.CommandBalance,
        WalletAddress: wallet.Address(),
    })

    if err != nil {
        onError(err.Error())
    }

    fmt.Printf("Balance: %f\n", response.Balance)
}

func ping() {
    _, err := node.SendIpc(node.Command {
        Kind: node.CommandPing,
    })

    if err != nil {
        onError(err.Error())
    }
}

func main() {
    newWalletCommand := flag.NewFlagSet("connect", flag.ExitOnError)
    outputPath := newWalletCommand.String("output", "", "Output file path")

    connectCommand := flag.NewFlagSet("connect", flag.ExitOnError)
    connectAddress := connectCommand.String("address", "", "Address to connect to")

    balanceCommand := flag.NewFlagSet("connect", flag.ExitOnError)
    walletPath := balanceCommand.String("wallet", "", "Wallet file path")

    if len(os.Args) < 2 {
        onError("Expected subcommand")
    }

    switch os.Args[1] {
    case "new-wallet":
        newWalletCommand.Parse(os.Args[2:])
        newWallet(*outputPath)
    case "connect":
        connectCommand.Parse(os.Args[2:])
        connect(*connectAddress)
    case "balance":
        balanceCommand.Parse(os.Args[2:])
        balance(*walletPath)
    case "ping":
        ping()
    default:
        onError("Unkown sub-command")
    }
}

