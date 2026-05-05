package main

import (
	"os"
)

func writeFile(path string, data []byte) {
	os.WriteFile(path, data, 0644)
}
