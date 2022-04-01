/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

package node

import (
	. "hyperchain/blockchain"
	"fmt"
	"os"
)

type Node struct {
    chain BlockChain
    rewardTo [32]byte

    network NetworkNode
    ipc chan Command
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

func (node *Node) handleCommand(command Command) {
    switch command.Kind {
    case CommandPing:
        fmt.Println("Ping")
        node.network.Send <- Packet { Kind: PacketPing }
    case CommandConnect:
        fmt.Printf("Connecting to '%s'\n", command.Address)
        node.network.ConnectPeer(command.Address)
    default:
        panic(command)
    }
}

func (node *Node) handleBlock(block Block) bool {
    if err := node.chain.Add(block); err != nil {
        fmt.Fprintf(os.Stderr, "Invalid block '%s'\n", err)
        return false
    }

    node.miner <- node.chain.NewBlock(node.rewardTo)
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
        case command := <- node.ipc:
            node.handleCommand(command)
        case block := <- node.miner:
            if !node.handleBlock(block) {
                node.miner <- node.chain.NewBlock(node.rewardTo)
            }
        }
    }
}

func StartNode(rewardTo [32]byte, port uint16) {
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

