package main

import "log"

func log_debug(msg string) {
	log.Printf("[DEBUG] %s", msg)
}

func log_info(msg string) {
	log.Printf("[INFO] %s", msg)
}

func log_warn(msg string) {
	log.Printf("[WARN] %s", msg)
}

func log_error(msg string) {
	log.Printf("[ERROR] %s", msg)
}

func log_fatal(msg string) {
	log.Fatalf("[FATAL] %s", msg)
}
