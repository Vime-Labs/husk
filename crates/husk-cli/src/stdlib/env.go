package main

import (
	"os"
	"strings"
)

func env_get(key string) string {
	return os.Getenv(key)
}

func env_get_or(key string, fallback string) string {
	val := os.Getenv(key)
	if val == "" {
		return fallback
	}
	return val
}

func env_require(key string) string {
	val := os.Getenv(key)
	if val == "" {
		panic("husk/env: variável obrigatória não definida: " + key)
	}
	return val
}

func env_load(paths ...string) {
	if len(paths) == 0 {
		paths = []string{".env"}
	}
	for _, p := range paths {
		data, err := os.ReadFile(p)
		if err != nil {
			continue
		}
		for _, line := range strings.Split(string(data), "\n") {
			line = strings.TrimSpace(line)
			if line == "" || strings.HasPrefix(line, "#") {
				continue
			}
			if parts := strings.SplitN(line, "=", 2); len(parts) == 2 {
				key := strings.TrimSpace(parts[0])
				val := strings.TrimSpace(parts[1])
				if os.Getenv(key) == "" {
					os.Setenv(key, val)
				}
			}
		}
	}
}
