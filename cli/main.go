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
	. "hyperchain/node"
	. "hyperchain/blockchain/wallet"
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

func connect(address string) {
    if address == "" {
        onError("No address given")
    }

    command := Command {
        Kind: CommandConnect,
        NodeAddress: address,
    }

    if _, err := SendIpc(command); err != "" {
        onError(err)
    }
}

func balance(walletPath string) {
    wallet, err := LoadWallet(walletPath)
    if err != nil {
        onError(err.Error())
    }

    address := wallet.Address()
    command := Command {
        Kind: CommandBalance,
        Address: address,
    }

    response, errMsg := SendIpc(command)
    if errMsg != "" {
        onError(errMsg)
    }

    fmt.Printf("Balance: %f\n", response.Balance)
}

func send(walletPath string, toAddress string, amount float32) {
    if walletPath == "" || toAddress == "" {
        onError("No wallet or to address given")
    }

    if amount <= 0 {
        onError("Invalid amount")
    }
    
    wallet, err := LoadWallet(walletPath)
    if err != nil {
        onError(err.Error())
    }

    to, err := DecodeAddress(toAddress)
    if err != nil {
        onError(err.Error())
    }

    command := Command {
        Kind: CommandSend,
        Wallet: wallet,
        Address: to,
        Amount: amount,
    }

    if _, errMsg := SendIpc(command); errMsg != "" {
        onError(errMsg)
    }
}

func ping() {
    command := Command {
        Kind: CommandPing,
    }

    if _, err := SendIpc(command); err != "" {
        onError(err)
    }
}

func main() {
    if len(os.Args) < 2 {
        onError("Expected subcommand")
    }

    switch os.Args[1] {
    case "new-wallet":
        command := flag.NewFlagSet("new-wallet", flag.ExitOnError)
        outputPath := command.String("output", "", "Output file path")
        command.Parse(os.Args[2:])

        newWallet(*outputPath)
    case "connect":
        command := flag.NewFlagSet("connect", flag.ExitOnError)
        connectAddress := command.String("address", "", "Address to connect to")
        command.Parse(os.Args[2:])

        connect(*connectAddress)
    case "balance":
        command := flag.NewFlagSet("balance", flag.ExitOnError)
        walletPath := command.String("wallet", "", "Wallet file path")
        command.Parse(os.Args[2:])

        balance(*walletPath)
    case "send":
        command := flag.NewFlagSet("send", flag.ExitOnError)
        walletPath := command.String("wallet", "", "Wallet file path")
        toAddress := command.String("to", "", "Address to send coins to")
        amount := command.Float64("amount", 0, "How many coins to send")
        command.Parse(os.Args[2:])

        send(*walletPath, *toAddress, float32(*amount))
    case "ping":
        ping()
    default:
        onError("Unkown sub-command")
    }
}

