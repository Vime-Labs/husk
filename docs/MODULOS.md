# Módulos

Cada arquivo `.husk` é um módulo. Módulos exportam funções e structs para outros arquivos via `import`.

## Sintaxe

```husk
import "caminho/do/modulo" as alias
```

O caminho é relativo ao arquivo que está importando. A extensão `.husk` é opcional.

## Exemplo

`usuarios.husk`:
```husk
struct Usuario {
    id   int
    nome string
}

fn listar() string {
    return "João, Maria, Pedro"
}

fn buscar(id int) string {
    return "usuario encontrado"
}
```

`main.husk`:
```husk
import "./usuarios" as usuarios

route GET /usuarios {
    return usuarios.listar()
}

route GET /usuarios/:id {
    return usuarios.buscar(42)
}
```

## Como funciona

O transpiler lê todos os arquivos importados, extrai suas funções e structs, e os inclui no mesmo `package main` do Go gerado. Por isso, a chamada `usuarios.listar()` é traduzida para `listar()` diretamente — o prefixo do alias é removido.

## O que é exportado

| Construto     | Exportado? |
|---------------|------------|
| `fn`          | sim        |
| `struct`      | sim        |
| `route`       | não — rotas só existem no arquivo raiz |
| `middleware`  | não — middlewares só existem no arquivo raiz |
| `import`      | não — imports não são transitivos |

## Importações circulares

Não são permitidas. O transpiler detecta ciclos e interrompe com erro.
