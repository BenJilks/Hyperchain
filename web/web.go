/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

package main

import (
	"fmt"
	"html/template"
	"log"
	"net/http"
)

var templates Templates

type Templates struct {
    index *template.Template
    block *template.Template
}

func main() {
    http.HandleFunc("/", index)
    http.HandleFunc("/block", block)

    templates = Templates {
        index: template.Must(template.ParseFiles("./web/templates/index.html")),
        block: template.Must(template.ParseFiles("./web/templates/block.html")),
    }

    fmt.Printf("Starting server at port 8000\n")
    if err := http.ListenAndServe(":8000", nil); err != nil {
        log.Fatal(err)
    }
}

