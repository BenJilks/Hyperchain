/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

package node

import (
	"fmt"
	. "hyperchain/blockchain"
	. "hyperchain/blockchain/transaction"
	. "hyperchain/blockchain/wallet"
)

type CommandKind int
const (
    CommandPing = CommandKind(iota)
    CommandConnect
    CommandBalance
    CommandSend
    CommandBlock
    CommandStats
)

type Command struct {
    Kind CommandKind

    NodeAddress string
    Address Address
    Wallet Wallet
    Amount float32
    ID int
}

func (ping *Command) ping(node *Node) (Response, error) {
    fmt.Println("Ping")
    node.network.Send <- Packet { Kind: PacketPing }
    return Response{}, nil
}

func (connect *Command) connect(node *Node) (Response, error) {
    fmt.Printf("Connecting to '%s'\n", connect.Address)
    node.network.ConnectPeer(connect.NodeAddress)
    return Response{}, nil
}

func (balance *Command) balance(node *Node) (Response, error) {
    fmt.Printf("Balance for '%s'\n", balance.Address.ToString())
    status, err := node.chain.WalletStatus(balance.Address)
    if err != nil {
        return Response{}, err
    }

    return Response { Balance: status.Balance }, nil
}

func (send *Command) send(node *Node) (Response, error) {
    fmt.Printf("Send %f from '%s' to '%s'\n",
        send.Amount,
        send.Wallet.Address().ToString(),
        send.Address.ToString())
    
    fee := float32(1.0)
    status, err := node.chain.WalletStatus(send.Wallet.Address())
    if err != nil {
        return Response{}, err
    }

    if status.Balance < send.Amount + fee {
        return Response{}, TransactionInsufficientInput
    }

    transaction, err := NewTransactionBuilder(status.LastId + 1, fee).
        AddInput(send.Wallet, send.Amount + fee).
        AddOutput(&Transfer { Address: send.Address, Amount: send.Amount }).
        Build()
    if err != nil {
        return Response{}, err
    }

    node.transactionQueue = append(node.transactionQueue, transaction)
    return Response{}, nil
}

func (block *Command) block(node *Node) (Response, error) {
    blockOrNone := node.chain.Block(block.ID)
    if blockOrNone == nil {
        return Response{}, fmt.Errorf("Unkown block %d", block.ID)
    }

    return Response { Block: *blockOrNone }, nil
}

func (stats *Command) stats(node *Node) (Response, error) {
    top := Block{}
    if block := node.chain.Top(); block != nil {
        top = *block
    }

    return Response {
        Block: top,
    }, nil
}

