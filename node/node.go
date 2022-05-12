/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

package node

import (
	"fmt"
	"os"
	. "hyperchain/blockchain"
	. "hyperchain/blockchain/wallet"
	. "hyperchain/blockchain/transaction"
)

type Node struct {
    chain BlockChain
    rewardTo Address
    transactionQueue []Transaction

    network NetworkNode
    ipc chan commandRequest
    miner chan Block
}

func (node *Node) handlePacket(packet Packet) {
    switch packet.Kind {
    case PacketHandShake:
        fmt.Println("Hand shake")
    case PacketPing:
        fmt.Println("Pong")
    case PacketBlock:
        node.handleBlock(packet.Block)
    default:
        panic(packet)
    }
}

func (node *Node) findTransactionInQueue(transaction *Transaction) int {
    for index, it := range node.transactionQueue {
        if it.Hash() == transaction.Hash() {
            return index
        }
    }

    return -1
}

func (node *Node) handleCommand(request commandRequest) {
    command := request.command
    var response Response
    var err error

    switch command.Kind {
    case CommandPing:
        response, err = command.ping(node)
    case CommandConnect:
        response, err = command.connect(node)
    case CommandBalance:
        response, err = command.balance(node)
    case CommandSend:
        response, err = command.send(node)
    default:
        panic(command.Kind)
    }

    if err != nil {
        request.response <- Response { Error: err.Error() }
    } else {
        request.response <- response
    }
}

func (node *Node) completedTransactions(transactions []Transaction) {
    for _, completed := range transactions {
        index := node.findTransactionInQueue(&completed)
        if index == -1 {
            continue
        }

        node.transactionQueue = append(
            node.transactionQueue[:index],
            node.transactionQueue[index+1:]...)
    }
}

func (node *Node) createBlock() Block {
    block := node.chain.NewBlock(node.rewardTo)

    // TODO: Limit how many transaction can be added
    block.Transactions = make([]Transaction, len(node.transactionQueue))
    copy(block.Transactions, node.transactionQueue)

    return block
}

func (node *Node) handleBlock(block Block) bool {
    if err := node.chain.Add(block); err != nil {
        fmt.Fprintf(os.Stderr, "Invalid block '%s'\n", err)
        return false
    }

    node.completedTransactions(block.Transactions)
    node.miner <- node.createBlock()
    node.network.Send <- Packet {
        Kind: PacketBlock,
        Block: block,
    }

    return true
}

func (node *Node) eventHandler() {
    for {
        select {
        case packet := <- node.network.Receive:
            node.handlePacket(packet)
        case request := <- node.ipc:
            node.handleCommand(request)
        case block := <- node.miner:
            if !node.handleBlock(block) {
                node.miner <- node.chain.NewBlock(node.rewardTo)
            }
        }
    }
}

func StartNode(rewardTo Address, port uint16) {
    node := Node {
        chain: NewBlockChain(),
        rewardTo: rewardTo,

        network: StartNetworkNode(port),
        ipc: ListenIpc(),
        miner: Miner(),
    }

    node.miner <- node.chain.NewBlock(rewardTo)
    node.eventHandler()
}

