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
)

type CommandKind int
const (
    CommandPing = CommandKind(iota)
    CommandConnect
)

type Command struct {
    Kind CommandKind
    Address string
}

func handleConnection(connection net.Conn, channel chan Command) {
    fmt.Println("Got IPC connection")
    reader := bufio.NewReader(connection)

    for {
        bytes, err := reader.ReadBytes('\n')
        if err == io.EOF {
            break
        }
        if err != nil {
            panic(err)
        }

        var command Command
        if err := json.Unmarshal(bytes, &command); err != nil {
            panic(err)
        }

        channel <- command
    }
}

func startIpcServer(channel chan Command) {
    listener, err := net.ListenUnix("unix", &net.UnixAddr {
        Name: "/tmp/hyperchain",
        Net: "unix",
    })
    if err != nil {
        panic(err)
    }

    for {
        connection, err := listener.Accept()
        if err != nil {
            panic(err)
        }

        go handleConnection(connection, channel)
    }
}

func ListenIpc() chan Command {
    channel := make(chan Command)

    go startIpcServer(channel)
    return channel
}

func SendIpc(command Command) error {
    sender, err := net.DialUnix("unix", nil, &net.UnixAddr {
        Name: "/tmp/hyperchain",
        Net: "unix",
    })
    if err != nil {
        return err
    }

    command_json, err := json.Marshal(command)
    if err != nil {
        return err
    }

    command_json = append(command_json, byte('\n'))
    if _, err := sender.Write(command_json); err != nil {
        return err
    }

    return nil
}

