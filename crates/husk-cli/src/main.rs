use husk_analyzer::analyze as analyze_semantic;
use husk_codegen::Codegen;
use husk_lexer::Lexer;
use husk_parser::{
    Parser,
    ast::{Item, Program},
};
use std::{
    collections::HashSet,
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::{self, Command},
    time::Instant,
};

// Shims da stdlib embutidos no binário
const STDLIB_ENV: &str = include_str!("stdlib/env.go");
const STDLIB_POSTGRES: &str = include_str!("stdlib/postgres.go");
const STDLIB_CRYPTO: &str = include_str!("stdlib/crypto.go");
const STDLIB_JWT: &str = include_str!("stdlib/jwt.go");

struct StdlibDeps {
    modules: Vec<String>,
}

impl StdlibDeps {
    fn from_program(program: &Program) -> Self {
        let modules = program
            .items
            .iter()
            .filter_map(|i| {
                if let Item::Import(imp) = i {
                    if imp.is_stdlib {
                        Some(imp.path.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        StdlibDeps { modules }
    }

    fn has(&self, name: &str) -> bool {
        self.modules.iter().any(|m| m == name)
    }

    fn go_mod_requires(&self) -> String {
        let mut reqs = vec!["github.com/go-chi/chi/v5 v5.2.1".to_string()];
        if self.has("husk/postgres") {
            reqs.push("github.com/jackc/pgx/v5 v5.7.4".to_string());
        }
        if self.has("husk/crypto") {
            reqs.push("golang.org/x/crypto v0.38.0".to_string());
        }
        if self.has("husk/jwt") {
            reqs.push("github.com/golang-jwt/jwt/v5 v5.2.2".to_string());
        }
        reqs.iter().map(|r| format!("\t{r}\n")).collect::<String>()
    }

    fn write_shims(&self, dir: &Path) {
        if self.has("husk/env") {
            write_file(&dir.join("husk_stdlib_env.go"), STDLIB_ENV);
        }
        if self.has("husk/postgres") {
            write_file(&dir.join("husk_stdlib_postgres.go"), STDLIB_POSTGRES);
        }
        if self.has("husk/crypto") {
            write_file(&dir.join("husk_stdlib_crypto.go"), STDLIB_CRYPTO);
        }
        if self.has("husk/jwt") {
            write_file(&dir.join("husk_stdlib_jwt.go"), STDLIB_JWT);
        }
    }
}

// ANSI sem dependências
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const GREEN: &str = "\x1b[32m";
const CYAN: &str = "\x1b[36m";
const RED: &str = "\x1b[31m";
const RESET: &str = "\x1b[0m";

fn main() {
    let args: Vec<String> = env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("run") => cmd_run(&args),
        Some("build") => cmd_build(&args),
        Some("check") => cmd_check(&args),
        Some("new") => cmd_new(&args),
        _ => {
            eprintln!("{BOLD}husk{RESET} — linguagem de programação web");
            eprintln!();
            eprintln!("{BOLD}uso:{RESET}");
            eprintln!("  husk run    <arquivo.husk>   transpila e executa");
            eprintln!("  husk build  <arquivo.husk>   gera binário Go");
            eprintln!("  husk check  <arquivo.husk>   verifica sintaxe");
            eprintln!("  husk new    <nome>            cria novo projeto");
            process::exit(1);
        }
    }
}

fn cmd_run(args: &[String]) {
    let file = require_file(args);
    let (go_code, stdlib) = transpile_file(file);
    let dir = prepare_go_dir(file, &go_code, &stdlib);

    step("dependências", "resolvendo...");
    let start = Instant::now();
    go_mod_tidy(&dir, file);
    ok(&format!(
        "dependências prontas {DIM}({:.1}s){RESET}",
        start.elapsed().as_secs_f32()
    ));

    step("servidor", "iniciando...");
    let status = Command::new("go")
        .args(["run", "."])
        .current_dir(&dir)
        .status()
        .unwrap_or_else(|_| die("'go' não encontrado. Instale em https://go.dev/dl/"));

    process::exit(status.code().unwrap_or(1));
}

fn cmd_build(args: &[String]) {
    let file = require_file(args);
    let stem = Path::new(file)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let (go_code, stdlib) = transpile_file(file);
    let dir = prepare_go_dir(file, &go_code, &stdlib);

    step("dependências", "resolvendo...");
    go_mod_tidy(&dir, file);

    step("compilando", &format!("{stem}..."));
    let start = Instant::now();
    let out_path = env::current_dir().unwrap().join(&stem);
    let status = Command::new("go")
        .args(["build", "-o", out_path.to_str().unwrap(), "."])
        .current_dir(&dir)
        .status()
        .expect("falha ao executar go build");

    if status.success() {
        ok(&format!(
            "binário gerado {BOLD}./{stem}{RESET} {DIM}({:.1}s){RESET}",
            start.elapsed().as_secs_f32()
        ));
    } else {
        process::exit(1);
    }
}

fn cmd_check(args: &[String]) {
    let file = require_file(args);
    let start = Instant::now();
    transpile_file(file); // check: só valida sintaxe/codegen, descarta saída
    ok(&format!(
        "{BOLD}{file}{RESET} {DIM}({:.0}ms){RESET}",
        start.elapsed().as_millis()
    ));
}

fn cmd_new(args: &[String]) {
    let name = args.get(2).unwrap_or_else(|| {
        eprintln!("{RED}erro:{RESET} informe o nome do projeto: husk new <nome>");
        process::exit(1);
    });

    let dir = Path::new(name);
    if dir.exists() {
        eprintln!("{RED}erro:{RESET} diretório '{name}' já existe");
        process::exit(1);
    }

    fs::create_dir_all(dir).expect("falha ao criar diretório");

    let main_husk = format!(
        "route GET /hello {{\n    return \"Hello from {name}!\"\n}}\n\nroute GET /ping {{\n    return json({{ status: \"ok\" }})\n}}\n"
    );
    fs::write(dir.join("main.husk"), main_husk).expect("falha ao criar main.husk");

    let gitignore = format!("{name}\n*.go\ngo.mod\ngo.sum\n");
    fs::write(dir.join(".gitignore"), gitignore).expect("falha ao criar .gitignore");

    ok(&format!("projeto {BOLD}{name}{RESET} criado"));
    eprintln!("{DIM}  cd {name} && husk run main.husk{RESET}");
}

// --- transpilação ---

fn transpile_file(file: &str) -> (String, StdlibDeps) {
    let step_start = Instant::now();
    step("transpilando", file);

    let source =
        fs::read_to_string(file).unwrap_or_else(|e| die(&format!("erro ao ler '{file}': {e}")));

    let base_dir = Path::new(file).parent().unwrap_or(Path::new("."));
    let program = parse_source(&source, file);
    let merged = resolve_imports(program, base_dir, &mut HashSet::new(), file);
    let stdlib = StdlibDeps::from_program(&merged);

    let go_code = Codegen::new()
        .generate(&merged)
        .unwrap_or_else(|e| die(&format!("{file}: erro de geração: {}", e.message)));

    ok(&format!(
        "{BOLD}{file}{RESET} → Go {DIM}({:.0}ms){RESET}",
        step_start.elapsed().as_millis()
    ));

    (go_code, stdlib)
}

fn parse_source(source: &str, file: &str) -> Program {
    let tokens = Lexer::new(source).tokenize().unwrap_or_else(|e| {
        die(&format!(
            "{BOLD}{file}:{}{RESET} erro léxico: {}",
            format_span(e.span.line, e.span.col),
            e.message
        ))
    });
    let program = Parser::new(tokens).parse().unwrap_or_else(|e| {
        die(&format!(
            "{BOLD}{file}:{}{RESET} erro de sintaxe: {}",
            format_span(e.span.line, e.span.col),
            e.message
        ))
    });

    // Análise semântica
    let sem_errors = analyze_semantic(&program);
    if !sem_errors.is_empty() {
        for err in &sem_errors {
            eprintln!(
                "{RED}{BOLD}{}:{}{RESET} erro semântico: {}",
                file,
                format_span(err.span.line, err.span.col),
                err.message
            );
        }
        process::exit(1);
    }

    program
}

fn resolve_imports(
    mut program: Program,
    base_dir: &Path,
    visited: &mut HashSet<PathBuf>,
    current_file: &str,
) -> Program {
    let imports: Vec<_> = program
        .items
        .iter()
        .filter_map(|i| {
            if let Item::Import(imp) = i {
                Some(imp.clone())
            } else {
                None
            }
        })
        .collect();

    for imp in imports {
        // imports stdlib são tratados via shims Go — não há arquivo .husk para resolver
        if imp.is_stdlib {
            continue;
        }

        let mut path = base_dir.join(&imp.path);
        if path.extension().is_none() {
            path.set_extension("husk");
        }

        let canonical = path.canonicalize().unwrap_or_else(|_| {
            die(&format!(
                "{current_file}: módulo não encontrado: '{}'",
                imp.path
            ))
        });

        if visited.contains(&canonical) {
            continue;
        }
        visited.insert(canonical.clone());

        let source = fs::read_to_string(&canonical).unwrap_or_else(|e| {
            die(&format!(
                "{current_file}: erro ao ler módulo '{}': {e}",
                imp.path
            ))
        });

        let mod_program = parse_source(&source, &canonical.to_string_lossy());
        let mod_dir = canonical.parent().unwrap_or(Path::new("."));
        let mod_program =
            resolve_imports(mod_program, mod_dir, visited, &canonical.to_string_lossy());

        for item in mod_program.items {
            match item {
                Item::FnDef(_) | Item::StructDef(_) => program.items.push(item),
                _ => {}
            }
        }
    }

    program
}

// --- Go helpers ---

fn prepare_go_dir(husk_file: &str, go_code: &str, stdlib: &StdlibDeps) -> PathBuf {
    let stem = Path::new(husk_file)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let dir = env::temp_dir().join(format!("husk_{stem}"));
    fs::create_dir_all(&dir).expect("falha ao criar diretório temporário");

    write_file(&dir.join("main.go"), go_code);

    let go_mod = format!(
        "module husk_out\n\ngo 1.21\n\nrequire (\n{})\n",
        stdlib.go_mod_requires()
    );
    write_file(&dir.join("go.mod"), &go_mod);

    stdlib.write_shims(&dir);

    dir
}

fn go_mod_tidy(dir: &Path, husk_file: &str) {
    let out = Command::new("go")
        .args(["mod", "tidy"])
        .current_dir(dir)
        .output()
        .unwrap_or_else(|_| die("'go' não encontrado. Instale em https://go.dev/dl/"));

    if !out.status.success() {
        eprintln!("{RED}erro:{RESET} falha ao resolver dependências Go:");
        eprintln!("{}", String::from_utf8_lossy(&out.stderr));
        let _ = husk_file; // usado em mensagens futuras de source map
        process::exit(1);
    }
}

fn write_file(path: &Path, content: &str) {
    let mut f = fs::File::create(path).expect("falha ao criar arquivo");
    f.write_all(content.as_bytes())
        .expect("falha ao escrever arquivo");
}

// --- output helpers ---

fn step(label: &str, msg: &str) {
    eprintln!("{CYAN}{BOLD}{label:>12}{RESET} {msg}");
}

fn ok(msg: &str) {
    eprintln!("{GREEN}{BOLD}{:>12}{RESET} {msg}", "✓");
}

fn die(msg: &str) -> ! {
    eprintln!("{RED}{BOLD}erro:{RESET} {msg}");
    process::exit(1);
}

fn format_span(line: usize, col: usize) -> String {
    format!("{line}:{col}")
}

fn require_file<'a>(args: &'a [String]) -> &'a str {
    args.get(2)
        .map(|s| s.as_str())
        .unwrap_or_else(|| die("informe o arquivo: husk run <arquivo.husk>"))
}
