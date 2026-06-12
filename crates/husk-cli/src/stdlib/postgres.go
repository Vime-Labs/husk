package main

import (
	"context"
	"encoding/hex"
	"fmt"
	"os"
	"sync"

	"github.com/jackc/pgx/v5/pgxpool"
)

func toUUIDString(b [16]byte) string {
	var buf [36]byte
	hex.Encode(buf[0:8], b[0:4])
	buf[8] = '-'
	hex.Encode(buf[9:13], b[4:6])
	buf[13] = '-'
	hex.Encode(buf[14:18], b[6:8])
	buf[18] = '-'
	hex.Encode(buf[19:23], b[8:10])
	buf[23] = '-'
	hex.Encode(buf[24:36], b[10:16])
	return string(buf[:])
}

func convertPgValue(v interface{}) interface{} {
	if b, ok := v.([16]byte); ok {
		return toUUIDString(b)
	}
	return v
}

var pgPool *pgxpool.Pool
var pgOnce sync.Once

func init() {
	url := os.Getenv("DATABASE_URL")
	if url == "" {
		return
	}
	if err := db_connect(url); err != nil {
		fmt.Printf("husk/postgres: erro ao conectar: %v\n", err)
	}
}

func db_connect(url string) error {
	var connectErr error
	pgOnce.Do(func() {
		pool, err := pgxpool.New(context.Background(), url)
		if err != nil {
			connectErr = err
			return
		}
		pgPool = pool
	})
	return connectErr
}

func db_query(sql string, args ...interface{}) ([]map[string]interface{}, error) {
	if pgPool == nil {
		return nil, fmt.Errorf("husk/postgres: sem conexão. Defina DATABASE_URL ou chame db.connect(url)")
	}
	rows, err := pgPool.Query(context.Background(), sql, args...)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	results := []map[string]interface{}{}
	for rows.Next() {
		values, err := rows.Values()
		if err != nil {
			return nil, err
		}
		row := make(map[string]interface{})
		for i, col := range rows.FieldDescriptions() {
			row[string(col.Name)] = convertPgValue(values[i])
		}
		results = append(results, row)
	}
	return results, rows.Err()
}

func db_query_one(sql string, args ...interface{}) (map[string]interface{}, error) {
	rows, err := db_query(sql, args...)
	if err != nil {
		return nil, err
	}
	if len(rows) == 0 {
		return nil, fmt.Errorf("husk/postgres: nenhum resultado encontrado")
	}
	return rows[0], nil
}

func db_exec(sql string, args ...interface{}) (interface{}, error) {
	if pgPool == nil {
		return nil, fmt.Errorf("husk/postgres: sem conexão. Defina DATABASE_URL ou chame db.connect(url)")
	}
	_, err := pgPool.Exec(context.Background(), sql, args...)
	return nil, err
}
