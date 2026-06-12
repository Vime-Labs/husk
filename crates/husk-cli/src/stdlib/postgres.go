package main

import (
	"context"
	"fmt"
	"os"
	"sync"

	"github.com/jackc/pgx/v5/pgxpool"
)

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
			row[string(col.Name)] = values[i]
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
