/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

package node

import (
	"bufio"
	"encoding/json"
	"fmt"
	"io"
	"net"
	"os"
)

type NetworkNode struct {
    newPeer chan net.Conn
    Send chan Packet
    Receive chan Packet
}

func (node *NetworkNode) ConnectPeer(address string) {
    connection, err := net.Dial("tcp4", address)
    if err != nil {
        panic(err)
    }

    node.newPeer <- connection
    go handlePeerConnection(connection, node.Receive)
}

func packetSender(newPeer chan net.Conn,
                  send chan Packet) {
    peers := make([]net.Conn, 0)
    for {
        select {
        case peer := <- newPeer:
            peers = append(peers, peer)
        case packet := <- send:
            packet_json, err := json.Marshal(packet)
            if err != nil {
                panic(err)
            }

            packet_json = append(packet_json, byte('\n'))
            for _, peer := range peers {
                if _, err := peer.Write(packet_json); err != nil {
                    panic(err)
                }
            }
        }
    }
}

func handlePeerConnection(connection net.Conn, receive chan Packet) {
    fmt.Printf("[%s] Connected to %s\n",
        connection.LocalAddr().String(),
        connection.RemoteAddr().String())

    reader := bufio.NewReader(connection)
    for {
        data, err := reader.ReadBytes('\n')
        if err == io.EOF {
            break
        }
        if err != nil {
            fmt.Fprintln(os.Stderr, err)
            break
        }

        packet := Packet {}
        if err := json.Unmarshal(data, &packet); err != nil {
            fmt.Fprintln(os.Stderr, err)
            break
        }

        receive <- packet
    }
}

func peerListener(listener net.Listener,
                  newPeer chan net.Conn,
                  receive chan Packet) {
    for {
        connection, err := listener.Accept()
        if err != nil {
            panic(err)
        }

        newPeer <- connection
        go handlePeerConnection(connection, receive)
    }
}

func StartNetworkNode(port uint16) NetworkNode {
    listener, err := net.Listen("tcp4", fmt.Sprintf("0.0.0.0:%d", port))
    if err != nil {
        panic(err)
    }

    newPeer := make(chan net.Conn)
    send := make(chan Packet)
    receive := make(chan Packet)
    go packetSender(newPeer, send)
    go peerListener(listener, newPeer, receive)

    return NetworkNode {
        newPeer: newPeer,
        Send: send,
        Receive: receive,
    }
}

