/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

package main

import (
	"net/http"
	"path"
)

type IndexData struct {
    Test string
}

func index(response http.ResponseWriter, request *http.Request) {
    if len(request.URL.Path) > 1 {
        path := path.Join("./web/static/", request.URL.Path[1:])
        http.ServeFile(response, request, path)
        return
    }

    templates.ExecuteTemplate(response, "Index", IndexData {
        Test: "This is a test",
    })
}

