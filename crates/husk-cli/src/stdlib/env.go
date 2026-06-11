package main

import "os"

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
