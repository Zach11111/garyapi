#!/bin/bash
go build -o api -ldflags "-s -w" src/main.go