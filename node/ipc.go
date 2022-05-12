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

type Response struct {
    Error string
    Balance float32
}

type commandRequest struct {
    command Command
    response chan Response
}

func handleConnection(connection net.Conn, channel chan commandRequest) {
    fmt.Println("Got IPC connection")
    reader := bufio.NewReader(connection)
    defer connection.Close()

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

        response := make(chan Response)
        channel <- commandRequest {
            command,
            response,
        }

        responseBytes, err := json.Marshal(<- response)
        if err != nil {
            panic(err)
        }

        responseBytes = append(responseBytes, '\n')
        if _, err = connection.Write(responseBytes); err != nil {
            panic(err)
        }
    }
}

func startIpcServer(channel chan commandRequest) {
    listener, err := net.ListenUnix("unix", &net.UnixAddr {
        Name: "/tmp/hyperchain",
        Net: "unix",
    })
    if err != nil {
        panic(err)
    }

    defer listener.Close()
    for {
        connection, err := listener.Accept()
        if err != nil {
            panic(err)
        }

        go handleConnection(connection, channel)
    }
}

func ListenIpc() chan commandRequest {
    channel := make(chan commandRequest)

    go startIpcServer(channel)
    return channel
}

func SendIpc(command Command) (Response, string) {
    sender, err := net.DialUnix("unix", nil, &net.UnixAddr {
        Name: "/tmp/hyperchain",
        Net: "unix",
    })

    if err != nil {
        return Response{}, err.Error()
    }
    defer sender.Close()

    command_json, err := json.Marshal(command)
    if err != nil {
        return Response{}, err.Error()
    }

    command_json = append(command_json, byte('\n'))
    if _, err := sender.Write(command_json); err != nil {
        return Response{}, err.Error()
    }
    
    reader := bufio.NewReader(sender)
    responseBytes, err := reader.ReadBytes('\n')
    if err != nil {
        return Response{}, err.Error()
    }

    var response Response
    if err = json.Unmarshal(responseBytes, &response); err != nil {
        return Response{}, err.Error()
    }

    if response.Error != "" {
        return Response{}, response.Error
    } else {
        return response, ""
    }
}

