/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

package main

import (
	"net/http"
	"strconv"
    "hyperchain/node"
)

type BlockData struct {
    ID uint64
    Top uint64
}

func block(w http.ResponseWriter, request *http.Request) {
    id, err := strconv.ParseInt(request.URL.Query().Get("id"), 10, 32)
    if err != nil {
        http.Redirect(w, request, "/", 301)
        return
    }

    response, errMsg := node.SendIpc(node.Command {
        Kind: node.CommandBlock,
        ID: int(id),
    })

    if errMsg != "" {
        http.Error(w, errMsg, 500)
        return
    }

    stats, errMsg := node.SendIpc(node.Command {
        Kind: node.CommandStats,
    })

    if errMsg != "" {
        http.Error(w, errMsg, 500)
        return
    }

    templates.block.Execute(w, BlockData {
        ID: response.Block.Id,
        Top: stats.Block.Id,
    })
}

