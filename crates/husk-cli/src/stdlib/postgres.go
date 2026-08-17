package main

import (
	"context"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"log"
	"net/url"
	"os"
	"sync"

	"github.com/jackc/pgx/v5/pgxpool"
)

// maskDSN esconde usuário/senha do DATABASE_URL para logar com segurança
// (host/porta/db seguem visíveis — úteis para confirmar qual banco o
// processo está tentando alcançar).
func maskDSN(dsn string) string {
	u, err := url.Parse(dsn)
	if err != nil {
		return "***"
	}
	if u.User != nil {
		u.User = url.User("***")
	}
	return u.String()
}

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

const (
	pgJSONOID  = 114
	pgJSONBOID = 3802
)

func convertPgValue(v interface{}, oid uint32) interface{} {
	if b, ok := v.([16]byte); ok {
		return toUUIDString(b)
	}
	if oid == pgJSONOID || oid == pgJSONBOID {
		var raw []byte
		switch t := v.(type) {
		case []byte:
			raw = t
		case string:
			raw = []byte(t)
		}
		if raw != nil {
			var decoded interface{}
			if err := json.Unmarshal(raw, &decoded); err == nil {
				return decoded
			}
			return string(raw)
		}
	}
	return v
}

var pgPool *pgxpool.Pool
var pgOnce sync.Once

func init() {
	dsn := os.Getenv("DATABASE_URL")
	if dsn == "" {
		log.Printf("[ERROR] husk/postgres: DATABASE_URL não definida no ambiente — o processo vai subir sem conexão com o banco")
		return
	}
	if err := db_connect(dsn); err != nil {
		log.Printf("[ERROR] husk/postgres: erro ao conectar em %s: %v", maskDSN(dsn), err)
		return
	}
	log.Printf("[INFO] husk/postgres: pool criado para %s", maskDSN(dsn))
}

func db_connect(dsn string) error {
	var connectErr error
	pgOnce.Do(func() {
		pool, err := pgxpool.New(context.Background(), dsn)
		if err != nil {
			connectErr = err
			return
		}
		// pgxpool.New não conecta de fato (lazy) — Ping força uma conexão
		// real agora, pra pegar erro de rede/DNS/credenciais no boot em vez
		// de só na primeira query de algum request.
		if err := pool.Ping(context.Background()); err != nil {
			connectErr = err
			pool.Close()
			return
		}
		pgPool = pool
	})
	return connectErr
}

func db_query(sql string, args ...interface{}) ([]map[string]interface{}, error) {
	if pgPool == nil {
		err := fmt.Errorf("husk/postgres: sem conexão. Defina DATABASE_URL ou chame db.connect(url)")
		log.Printf("[ERROR] %v — query: %s", err, sql)
		return nil, err
	}
	rows, err := pgPool.Query(context.Background(), sql, args...)
	if err != nil {
		log.Printf("[ERROR] husk/postgres: query falhou (%v) — sql: %s", err, sql)
		return nil, err
	}
	defer rows.Close()

	results := []map[string]interface{}{}
	for rows.Next() {
		values, err := rows.Values()
		if err != nil {
			log.Printf("[ERROR] husk/postgres: erro lendo linha (%v) — sql: %s", err, sql)
			return nil, err
		}
		row := make(map[string]interface{})
		for i, col := range rows.FieldDescriptions() {
			row[string(col.Name)] = convertPgValue(values[i], col.DataTypeOID)
		}
		results = append(results, row)
	}
	if err := rows.Err(); err != nil {
		log.Printf("[ERROR] husk/postgres: erro após iterar linhas (%v) — sql: %s", err, sql)
		return nil, err
	}
	return results, nil
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
		err := fmt.Errorf("husk/postgres: sem conexão. Defina DATABASE_URL ou chame db.connect(url)")
		log.Printf("[ERROR] %v — exec: %s", err, sql)
		return nil, err
	}
	_, err := pgPool.Exec(context.Background(), sql, args...)
	if err != nil {
		log.Printf("[ERROR] husk/postgres: exec falhou (%v) — sql: %s", err, sql)
	}
	return nil, err
}
