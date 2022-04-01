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
)

func main() {
    connectCommand := flag.NewFlagSet("connect", flag.ExitOnError)
    connectAddress := connectCommand.String("address", "", "Address to connect to")

    if len(os.Args) < 2 {
        fmt.Fprintln(os.Stderr, "Expected subcommand")
        os.Exit(1)
    }

    switch os.Args[1] {
    case "connect":
        connectCommand.Parse(os.Args[2:])
        if *connectAddress == "" {
            flag.Usage()
            os.Exit(1)
        }

        err := node.SendIpc(node.Command {
            Kind: node.CommandConnect,
            Address: *connectAddress,
        })

        if err != nil {
            panic(err)
        }
    case "ping":
        err := node.SendIpc(node.Command {
            Kind: node.CommandPing,
        })

        if err != nil {
            panic(err)
        }
    default:
        flag.Usage()
    }
}

