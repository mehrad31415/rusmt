// Runner: BurntSushi/toml. Prints `OK` or `ERR <class>`.
package main

import (
	"fmt"
	"os"
	"strings"

	"github.com/BurntSushi/toml"
)

func main() {
	var v map[string]any
	if _, err := toml.DecodeFile(os.Args[1], &v); err != nil {
		m := strings.ReplaceAll(err.Error(), "\n", " ")
		if len(m) > 60 {
			m = m[:60]
		}
		fmt.Println("ERR " + m)
		return
	}
	fmt.Println("OK")
}
