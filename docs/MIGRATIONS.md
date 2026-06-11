# Migrations

Migrations gerenciam a evolução do schema do banco de dados ao longo do tempo. Cada migration é um arquivo SQL versionado com instruções de aplicação (`Up`) e reversão (`Down`).

Husk usa [goose](https://github.com/pressly/goose) como engine de migrations, mas você não precisa instalá-lo — ele é invocado automaticamente pelo CLI.

## Pré-requisitos

- Módulo `husk/postgres` adicionado ao projeto (`husk add postgres`)
- Variável `DATABASE_URL` definida no `.env` ou no ambiente

```
DATABASE_URL=postgres://usuario:senha@localhost:5432/meu_banco
```

## Comandos

### `husk migrate create <nome>`

Cria um novo arquivo de migration em `migrations/` com timestamp no nome:

```
$ husk migrate create create_usuarios
✓  migrations/20260611143022_create_usuarios.sql
```

O diretório `migrations/` é criado automaticamente se não existir.

### `husk migrate up`

Aplica todas as migrations pendentes em ordem:

```
$ husk migrate up
     migrações  up...
2026/06/11 14:30:22 OK   20260611143022_create_usuarios.sql (12.34ms)
2026/06/11 14:30:22 OK   20260611150000_add_email_usuarios.sql (3.21ms)
✓  migrate up concluído
```

### `husk migrate down`

Reverte a última migration aplicada:

```
$ husk migrate down
     migrações  down...
2026/06/11 14:31:00 OK   20260611150000_add_email_usuarios.sql (2.11ms)
✓  migrate down concluído
```

### `husk migrate status`

Lista todas as migrations e seu estado (aplicada ou pendente):

```
$ husk migrate status
     migrações  status...
    Applied At                  Migration
    =======================================
    2026/06/11 14:30:22 -- 20260611143022_create_usuarios.sql
    Pending                  -- 20260611150000_add_email_usuarios.sql
```

## Formato do arquivo SQL

Cada arquivo usa anotações `-- +goose` para separar as seções de aplicação e reversão:

```sql
-- +goose Up
CREATE TABLE usuarios (
    id         SERIAL PRIMARY KEY,
    nome       TEXT        NOT NULL,
    email      TEXT        NOT NULL UNIQUE,
    criado_em  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- +goose Down
DROP TABLE usuarios;
```

A seção `Up` é executada em `husk migrate up`. A seção `Down` é executada em `husk migrate down`.

## Fluxo típico

```
# 1. criar a migration
husk migrate create create_usuarios

# 2. editar migrations/20260611143022_create_usuarios.sql

# 3. aplicar
husk migrate up

# 4. verificar estado
husk migrate status
```

## Múltiplas migrations

Migrations são aplicadas em ordem crescente de timestamp. O goose rastreia quais já foram aplicadas em uma tabela `goose_db_version` criada automaticamente no banco.

```
migrations/
  20260601000000_create_usuarios.sql
  20260605120000_add_perfil_usuarios.sql
  20260611143022_create_produtos.sql   ← pendente
```

`husk migrate up` aplica apenas as pendentes — no exemplo acima, só a terceira.

## Transações

Cada migration roda em uma transação por padrão. Se qualquer instrução falhar, toda a migration é revertida e o banco permanece no estado anterior.

Para desabilitar a transação em uma migration específica (ex: para `CREATE INDEX CONCURRENTLY`):

```sql
-- +goose Up
-- +goose NO TRANSACTION
CREATE INDEX CONCURRENTLY idx_usuarios_email ON usuarios(email);

-- +goose Down
DROP INDEX idx_usuarios_email;
```

## Integração com `.env`

O `.env` do projeto é carregado automaticamente. Não é necessário exportar `DATABASE_URL` manualmente:

```
# .env
DATABASE_URL=postgres://usuario:senha@localhost:5432/meu_banco
```

```
$ husk migrate up   # lê DATABASE_URL do .env automaticamente
```

## Primeira execução

Na primeira vez que `husk migrate up/down/status` é chamado, o CLI baixa as dependências Go necessárias (`pressly/goose` e `jackc/pgx`). Isso leva alguns segundos. As execuções seguintes são rápidas porque as dependências ficam em cache.
