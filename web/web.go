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

var templates *template.Template

func main() {
    http.HandleFunc("/", index)
    http.HandleFunc("/block", block)

    templates = template.Must(template.ParseFiles(
        "./web/templates/index.html",
        "./web/templates/block.html",
        "./web/templates/transaction-data.html",
    ))

    fmt.Printf("Starting server at port 8000\n")
    if err := http.ListenAndServe(":8000", nil); err != nil {
        log.Fatal(err)
    }
}

