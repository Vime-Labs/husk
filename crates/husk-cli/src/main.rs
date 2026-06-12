use husk_analyzer::analyze as analyze_semantic;
use husk_codegen::Codegen;
use husk_lexer::Lexer;
use husk_parser::{
    Parser,
    ast::{Item, Program},
    formatter,
};
use serde::Deserialize;
use std::{
    collections::HashSet,
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::{self, Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime},
};

// Shims da stdlib embutidos no binário
const STDLIB_ENV: &str = include_str!("stdlib/env.go");
const STDLIB_POSTGRES: &str = include_str!("stdlib/postgres.go");
const STDLIB_CRYPTO: &str = include_str!("stdlib/crypto.go");
const STDLIB_JWT: &str = include_str!("stdlib/jwt.go");
const STDLIB_LOG: &str = include_str!("stdlib/log.go");
const STDLIB_HTTP: &str = include_str!("stdlib/http.go");
const MIGRATE_GO: &str = include_str!("stdlib/migrate.go");
const VENDOR_HUSK: &str = ".vendor.husk";

#[derive(Deserialize, Clone)]
struct Dependency {
    git: String,
    #[serde(default)]
    r#ref: String,
}

#[derive(Deserialize)]
struct Manifest {
    name: Option<String>,
    #[serde(default)]
    dependencies: std::collections::HashMap<String, Dependency>,
}

struct StdlibDeps {
    modules: Vec<String>,
    has_cors: bool,
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
        let has_cors = program.items.iter().any(|i| matches!(i, Item::CorsDef(_)));
        StdlibDeps { modules, has_cors }
    }

    fn has(&self, name: &str) -> bool {
        self.modules.iter().any(|m| m == name)
    }

    fn go_mod_requires(&self) -> String {
        let mut reqs = vec!["github.com/go-chi/chi/v5 v5.2.1".to_string()];
        if self.has_cors {
            reqs.push("github.com/go-chi/cors v1.2.2".to_string());
        }
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
        if self.has("husk/log") {
            write_file(&dir.join("husk_stdlib_log.go"), STDLIB_LOG);
        }
        if self.has("husk/http") {
            write_file(&dir.join("husk_stdlib_http.go"), STDLIB_HTTP);
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
        Some("lsp") => cmd_lsp(),
        Some("test") => cmd_test(&args),
        Some("dev") => cmd_dev(&args),
        Some("fmt") => cmd_fmt(&args),
        Some("add") => cmd_add(&args),
        Some("new") => cmd_new(&args),
        Some("install") => cmd_install(&args),
        Some("migrate") => cmd_migrate(&args),
        _ => {
            eprintln!("{BOLD}husk{RESET} — linguagem de programação web");
            eprintln!();
            eprintln!("{BOLD}uso:{RESET}");
            eprintln!("  husk lsp                           inicia servidor LSP");
            eprintln!("  husk run    <arquivo.husk>        transpila e executa");
            eprintln!("  husk dev    <arquivo.husk>        hot reload (transpila e reinicia ao salvar)");
            eprintln!("  husk build  <arquivo.husk>        gera binário Go");
            eprintln!("  husk test   [arquivo]              executa testes");
            eprintln!("  husk check  <arquivo.husk>        verifica sintaxe");
            eprintln!("  husk fmt    <arquivo.husk>        formata código");
            eprintln!("  husk add    <modulo>               adiciona módulo stdlib");
            eprintln!("  husk new    <nome>                 cria novo projeto");
            eprintln!("  husk install                       instala dependências (vendor/)");
            eprintln!("  husk migrate create <nome>        cria migration SQL");
            eprintln!("  husk migrate up                   aplica migrations pendentes");
            eprintln!("  husk migrate down                 reverte a última migration");
            eprintln!("  husk migrate status               lista estado das migrations");
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
    // Usa status() em vez de output() para que stdout/stderr do Go
    // vá direto para o terminal (servidor web nunca termina sozinho).
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
    let output = Command::new("go")
        .args(["build", "-o", out_path.to_str().unwrap(), "."])
        .current_dir(&dir)
        .output()
        .expect("falha ao executar go build");

    if output.status.success() {
        ok(&format!(
            "binário gerado {BOLD}./{stem}{RESET} {DIM}({:.1}s){RESET}",
            start.elapsed().as_secs_f32()
        ));
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let translated = translate_go_errors(&stderr, &dir.join("main.go"), file);
        eprint!("{}", translated);
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

fn cmd_fmt(args: &[String]) {
    let file = require_file(args);
    let start = Instant::now();

    let source =
        fs::read_to_string(file).unwrap_or_else(|e| die(&format!("erro ao ler '{file}': {e}")));

    let tokens = Lexer::new(&source).tokenize().unwrap_or_else(|e| {
        die(&format!(
            "{}:{}: erro léxico: {}",
            file,
            format_span(e.span.line, e.span.col),
            e.message
        ))
    });
    let program = Parser::new(tokens).parse().unwrap_or_else(|e| {
        die(&format!(
            "{}:{}: erro de sintaxe: {}",
            file,
            format_span(e.span.line, e.span.col),
            e.message
        ))
    });

    let formatted = formatter::format_program_with_source(&program, &source);
    fs::write(file, &formatted).unwrap_or_else(|e| die(&format!("erro ao escrever '{file}': {e}")));

    ok(&format!(
        "{BOLD}{file}{RESET} formatado {DIM}({:.0}ms){RESET}",
        start.elapsed().as_millis()
    ));
}

fn cmd_add(args: &[String]) {
    let module = args.get(2).unwrap_or_else(|| {
        eprintln!("{RED}erro:{RESET} informe o módulo: husk add <modulo>");
        eprintln!("  módulos disponíveis: env, log, postgres, crypto, jwt, http");
        process::exit(1);
    });

    let available = ["env", "log", "postgres", "crypto", "jwt", "http"];
    if !available.contains(&module.as_str()) {
        eprintln!(
            "{RED}erro:{RESET} módulo desconhecido '{module}'",
        );
        eprintln!("  disponíveis: env, log, postgres, crypto, jwt, http");
        process::exit(1);
    }

    // procura o arquivo .husk no diretório atual
    let cwd = env::current_dir().unwrap();
    let entries = fs::read_dir(&cwd).unwrap();
    let husk_file: Option<String> = entries
        .filter_map(|e| e.ok())
        .find(|e| {
            e.path().extension().is_some_and(|ext| ext == "husk")
                && e.file_type().is_ok_and(|t| t.is_file())
        })
        .map(|e| e.path().to_string_lossy().to_string());

    let file = husk_file.unwrap_or_else(|| {
        die("nenhum arquivo .husk encontrado no diretório atual")
    });

    let source = fs::read_to_string(&file).unwrap_or_else(|e| die(&format!("erro ao ler '{file}': {e}")));

    let import_line = format!("import \"husk/{module}\" as {module}\n");

    // só adiciona se ainda não existe
    if source.contains(&import_line) {
        ok(&format!("'{module}' já está no projeto"));
        return;
    }

    let new_source = import_line + &source;
    fs::write(&file, &new_source).unwrap_or_else(|e| die(&format!("erro ao escrever '{file}': {e}")));

    ok(&format!("{BOLD}{module}{RESET} adicionado a {file}"));
}

fn cmd_install(args: &[String]) {
    let force = args.contains(&"--force".to_string());
    let cwd = env::current_dir().unwrap_or_else(|_| die("erro ao obter diretório atual"));
    let manifest_path = cwd.join("husk.json");

    if !manifest_path.exists() {
        die("husk.json não encontrado. Crie um com 'husk new <nome>'");
    }

    let manifest_str = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| die(&format!("erro ao ler husk.json: {e}")));
    let manifest: Manifest = serde_json::from_str(&manifest_str)
        .unwrap_or_else(|e| die(&format!("erro ao fazer parse de husk.json: {e}")));

    if manifest.dependencies.is_empty() {
        ok("nenhuma dependência declarada em husk.json");
        return;
    }

    let vendor_dir = cwd.join("vendor");
    fs::create_dir_all(&vendor_dir).expect("falha ao criar diretório vendor/");

    let mut installed = std::collections::HashSet::new();
    step("instalando", &format!("{} dependências", manifest.dependencies.len()));

    for (name, dep) in &manifest.dependencies {
        install_dep(name, dep, &vendor_dir, &mut installed, force);
    }

    generate_vendor_husk(&manifest, &cwd);
    ok(&format!("{BOLD}{}{RESET} dependências instaladas em vendor/", manifest.dependencies.len()));
}

const VENDOR_HUSK_COMMENT: &str = "// gerado automaticamente por 'husk install' — não edite\n";

fn install_dep(
    name: &str,
    dep: &Dependency,
    vendor_dir: &Path,
    installed: &mut std::collections::HashSet<String>,
    force: bool,
) {
    let dep_dir = vendor_dir.join(name);
    let dep_key = format!("{}@{}", dep.git, dep.r#ref);

    if installed.contains(&dep_key) {
        return;
    }
    installed.insert(dep_key);

    if dep_dir.exists() {
        if force {
            fs::remove_dir_all(&dep_dir).expect("falha ao remover vendor existente");
        } else {
            step("já instalado", name);
            return;
        }
    }

    step("clonando", name);
    let ref_spec = if dep.r#ref.is_empty() {
        "HEAD".to_string()
    } else {
        dep.r#ref.clone()
    };

    let status = Command::new("git")
        .args([
            "clone",
            "--depth", "1",
            "--branch", &ref_spec,
            &dep.git,
            &dep_dir.to_string_lossy(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap_or_else(|_| die("git não encontrado. Instale em https://git-scm.com/"));

    if !status.success() {
        // tenta sem branch (pode ser commit hash)
        let status2 = Command::new("git")
            .args([
                "clone",
                "--depth", "1",
                &dep.git,
                &dep_dir.to_string_lossy(),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap_or_else(|_| die("falha ao executar git clone"));

        if !status2.success() {
            die(&format!("falha ao clonar '{}' de '{}'", name, dep.git));
        }

        // faz checkout do ref específico (se for commit hash)
        if !dep.r#ref.is_empty() {
            let co = Command::new("git")
                .args(["-C", &dep_dir.to_string_lossy(), "checkout", &dep.r#ref])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap();
            if !co.success() {
                die(&format!("falha ao fazer checkout de '{}'", dep.r#ref));
            }
        }
    }

    // dependências transitivas
    let trans_manifest_path = dep_dir.join("husk.json");
    if trans_manifest_path.exists() {
        let trans_str = fs::read_to_string(&trans_manifest_path)
            .unwrap_or_else(|e| die(&format!("erro ao ler husk.json de '{}': {e}", name)));
        if let Ok(trans_manifest) = serde_json::from_str::<Manifest>(&trans_str) {
            for (trans_name, trans_dep) in &trans_manifest.dependencies {
                let trans_dir = vendor_dir.join(trans_name);
                if !trans_dir.exists() || force {
                    install_dep(trans_name, trans_dep, vendor_dir, installed, force);
                }
            }
        }
    }
}

fn generate_vendor_husk(manifest: &Manifest, cwd: &Path) {
    let mut lines = vec![VENDOR_HUSK_COMMENT.to_string()];
    for (name, _dep) in &manifest.dependencies {
        let entry_point = find_entry_point(&cwd.join("vendor").join(name));
        lines.push(format!("import \"./{entry_point}\" as {name}\n"));
    }
    let content: String = lines.join("");
    let vendor_husk_path = cwd.join(VENDOR_HUSK);
    fs::write(&vendor_husk_path, &content)
        .unwrap_or_else(|e| die(&format!("erro ao escrever .vendor.husk: {e}")));
}

fn find_entry_point(dir: &Path) -> String {
    let candidates = ["main.husk", "mod.husk", "lib.husk"];
    for name in &candidates {
        let path = dir.join(name);
        if path.exists() {
            return format!("vendor/{}/{}", dir.file_name().unwrap().to_string_lossy(), name);
        }
    }
    // fallback: primeiro .husk que encontrar
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "husk") {
                let fname = path.file_name().unwrap().to_string_lossy();
                return format!("vendor/{}/{}", dir.file_name().unwrap().to_string_lossy(), fname);
            }
        }
    }
    format!("vendor/{}/main.husk", dir.file_name().unwrap().to_string_lossy())
}

fn cmd_lsp() {
    let bin = env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .join("husk-lsp");
    if !bin.exists() {
        die("binário husk-lsp não encontrado. Compile com: cargo build -p husk-lsp");
    }
    let mut output = Command::new(&bin)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap_or_else(|e| die(&format!("falha ao iniciar husk-lsp: {e}")));
    let status = output.wait().unwrap_or_else(|e| die(&format!("erro no husk-lsp: {e}")));
    process::exit(status.code().unwrap_or(1));
}

fn cmd_test(args: &[String]) {
    let start = Instant::now();

    // descobre arquivos de teste
    let test_files: Vec<(String, String)> = if let Some(file) = args.get(2) {
        let path = Path::new(file);
        if path.is_dir() {
            discover_test_files(path)
        } else {
            let source = fs::read_to_string(file).unwrap_or_else(|e| die(&format!("erro ao ler '{file}': {e}")));
            vec![(file.clone(), source)]
        }
    } else {
        discover_test_files(&env::current_dir().unwrap())
    };

    if test_files.is_empty() {
        die("nenhum arquivo _test.husk encontrado");
    }

    step("testes", &format!("{} arquivo(s)", test_files.len()));

    // transpila cada arquivo e coleta funções test_*
    let mut test_fns: Vec<(String, String)> = Vec::new(); // (nome, go_code da fn)
    let mut all_go_code = String::new();
    let mut stdlib = StdlibDeps { modules: Vec::new(), has_cors: false };

    for (file, source) in &test_files {
        step("  ", file);
        let base_dir = Path::new(file).parent().unwrap_or(Path::new("."));
        let program = parse_source(source, file);
        let mut visited = HashSet::new();
        let mut processing = HashSet::new();
        let merged = resolve_imports(program, base_dir, &mut visited, &mut processing, file);
        stdlib = StdlibDeps::from_program(&merged);

        let mut codegen = Codegen::new();
        let go = codegen.generate(&merged).unwrap_or_else(|e| die(&format!("{file}: erro de geração: {}", e.message)));

        // extrai funções test_* do Go gerado
        let test_harness = extract_test_functions(&go, file);
        test_fns.extend(test_harness);
        all_go_code.push_str(&go);
        all_go_code.push('\n');
    }

    if test_fns.is_empty() {
        die("nenhuma função test_* encontrada nos arquivos de teste");
    }

    // gera runner Go
    let runner = generate_test_runner(&test_fns);
    all_go_code.push_str(&runner);

    let dir = env::temp_dir().join(format!("husk_test_{}", process::id()));
    fs::create_dir_all(&dir).expect("falha ao criar diretório temporário");

    write_file(&dir.join("main.go"), &all_go_code);
    let go_mod = format!(
        "module husk_test\ngo 1.21\n\nrequire (\n{}\n)\n",
        stdlib.go_mod_requires()
    );
    write_file(&dir.join("go.mod"), &go_mod);
    stdlib.write_shims(&dir);

    go_mod_tidy(&dir, "test");

    step("executando", "testes...");
    let output = Command::new("go")
        .args(["run", "."])
        .current_dir(&dir)
        .output()
        .unwrap_or_else(|_| die("'go' não encontrado"));

    // mostra resultados
    println!("{}", String::from_utf8_lossy(&output.stdout));
    if !output.stderr.is_empty() {
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    }

    if output.status.success() {
        ok(&format!("todos os testes passaram {DIM}({:.1}s){RESET}", start.elapsed().as_secs_f32()));
    } else {
        eprintln!("{RED}{} teste(s) falharam{RESET}", test_fns.len());
        process::exit(1);
    }
}

/// Descobre arquivos *_test.husk em um diretório
fn discover_test_files(dir: &Path) -> Vec<(String, String)> {
    let mut files = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return files,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "husk")
            && path
                .file_stem()
                .map(|s| s.to_string_lossy().ends_with("_test"))
                .unwrap_or(false)
        {
            if let Ok(source) = fs::read_to_string(&path) {
                files.push((path.to_string_lossy().to_string(), source));
            }
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

/// Extrai funções test_* do Go gerado e retorna (nome, código_go)
fn extract_test_functions(go_code: &str, _file: &str) -> Vec<(String, String)> {
    let mut fns = Vec::new();
    let mut lines = go_code.lines().peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("func test_") {
            // pega o nome até o (
            if let Some(name_end) = rest.find('(') {
                let name = format!("test_{}", &rest[..name_end]);
                // coleta o corpo da função
                let mut body = String::from(line);
                body.push('\n');
                let opens = line.bytes().filter(|&b| b == b'{').count();
                let closes = line.bytes().filter(|&b| b == b'}').count();
                let mut brace_count = opens as isize - closes as isize;

                while brace_count > 0 {
                    if let Some(next) = lines.next() {
                        body.push_str(next);
                        body.push('\n');
                        let o = next.bytes().filter(|&b| b == b'{').count() as isize;
                        let c = next.bytes().filter(|&b| b == b'}').count() as isize;
                        brace_count += o - c;
                    } else {
                        break;
                    }
                }
                fns.push((name, body));
            }
        }
    }
    fns
}

/// Gera o runner Go que chama as funções test_*
fn generate_test_runner(test_fns: &[(String, String)]) -> String {
    let mut out = String::new();
    out.push_str("func main() {\n");
    out.push_str("    failed := false\n");
    out.push_str("    report := func(name string, fn func()) {\n");
    out.push_str("        defer func() {\n");
    out.push_str("            if r := recover(); r != nil {\n");
    out.push_str("                fmt.Printf(\"  FAIL  %s\\n  %v\\n\", name, r)\n");
    out.push_str("                failed = true\n");
    out.push_str("            }\n");
    out.push_str("        }()\n");
    out.push_str("        fn()\n");
    out.push_str("        fmt.Printf(\"  PASS  %s\\n\", name)\n");
    out.push_str("    }\n");
    out.push('\n');

    // inclui o código das funções test_*
    for (_, body) in test_fns {
        out.push_str(body);
        out.push('\n');
    }

    // chama cada função test_
    for (name, _) in test_fns {
        out.push_str(&format!("    report(\"{}\", {})\n", name, name));
    }

    out.push('\n');
    out.push_str("    if failed {\n");
    out.push_str("        os.Exit(1)\n");
    out.push_str("    }\n");
    out.push_str("}\n");
    out
}

fn cmd_dev(args: &[String]) {
    let file = require_file(args);
    let file_path = Path::new(file).to_path_buf();
    let file_stem = file_path.file_stem().unwrap_or_default().to_string_lossy().to_string();

    step("dev", &format!("monitorando {file}..."));

    let mut last_mtime = fs::metadata(&file_path)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);

    // transpila e inicia
    let mut server = start_dev_server(file);

    loop {
        thread::sleep(Duration::from_millis(500));

        let current_mtime = match fs::metadata(&file_path).and_then(|m| m.modified()) {
            Ok(t) => t,
            Err(_) => continue,
        };

        if current_mtime == last_mtime {
            continue;
        }
        last_mtime = current_mtime;

        // arquivo mudou — reinicia
        step("dev", "alteração detectada, reiniciando...");
        if let Some(ref mut child) = server {
            let _ = child.kill();
            let _ = child.wait();
        }
        server = start_dev_server(file);
    }
}

fn start_dev_server(file: &str) -> Option<process::Child> {
    let (go_code, stdlib) = transpile_file(file);
    let dir = prepare_go_dir(file, &go_code, &stdlib);
    go_mod_tidy(&dir, file);

    match Command::new("go")
        .args(["run", "."])
        .current_dir(&dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
    {
        Ok(child) => {
            eprintln!("  servidor iniciado (pid {})", child.id());
            Some(child)
        }
        Err(e) => {
            eprintln!("  {RED}erro:{RESET} falha ao iniciar servidor: {e}");
            None
        }
    }
}

fn cmd_migrate(args: &[String]) {
    let subcmd = args.get(2).map(|s| s.as_str()).unwrap_or_else(|| {
        eprintln!("{RED}erro:{RESET} uso: husk migrate <create|up|down|status> [nome]");
        process::exit(1);
    });

    match subcmd {
        "create" => migrate_create(args),
        "up" | "down" | "status" => migrate_run(subcmd),
        _ => {
            eprintln!("{RED}erro:{RESET} subcomando desconhecido '{subcmd}'");
            eprintln!("  disponíveis: create, up, down, status");
            process::exit(1);
        }
    }
}

fn migrate_create(args: &[String]) {
    let name = args.get(3).map(|s| s.as_str()).unwrap_or_else(|| {
        eprintln!("{RED}erro:{RESET} uso: husk migrate create <nome>");
        process::exit(1);
    });

    let cwd = env::current_dir().unwrap();
    let migrations_dir = cwd.join("migrations");
    fs::create_dir_all(&migrations_dir).expect("falha ao criar diretório migrations/");

    let ts = unix_to_timestamp(
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    );
    let filename = format!("{}_{}.sql", ts, name);
    let content = "-- +goose Up\n\n\n-- +goose Down\n\n";
    fs::write(migrations_dir.join(&filename), content).expect("falha ao criar migration");

    ok(&format!("migrations/{filename}"));
}

fn migrate_run(subcmd: &str) {
    let cwd = env::current_dir().unwrap();
    let migrations_dir = cwd.join("migrations");

    if !migrations_dir.exists() {
        eprintln!("{RED}erro:{RESET} diretório 'migrations/' não encontrado");
        eprintln!("  crie uma migration: husk migrate create <nome>");
        process::exit(1);
    }

    let dir = env::temp_dir().join("husk_migrate");
    fs::create_dir_all(&dir).expect("falha ao criar diretório temporário");

    write_file(&dir.join("main.go"), MIGRATE_GO);

    // só inicializa go.mod+deps se ainda não tiver goose (cache entre execuções)
    let go_mod_path = dir.join("go.mod");
    let needs_init = !go_mod_path.exists()
        || !fs::read_to_string(&go_mod_path)
            .unwrap_or_default()
            .contains("pressly/goose");

    if needs_init {
        write_file(&go_mod_path, "module husk_migrate\n\ngo 1.21\n");

        step("dependências", "inicializando (apenas na primeira vez)...");

        for dep in &[
            "github.com/pressly/goose/v3@latest",
            "github.com/jackc/pgx/v5@v5.7.4",
        ] {
            let out = Command::new("go")
                .args(["get", dep])
                .current_dir(&dir)
                .output()
                .unwrap_or_else(|_| die("'go' não encontrado. Instale em https://go.dev/dl/"));
            if !out.status.success() {
                eprintln!("{RED}erro:{RESET} {}", String::from_utf8_lossy(&out.stderr));
                process::exit(1);
            }
        }

        let tidy = Command::new("go")
            .args(["mod", "tidy"])
            .current_dir(&dir)
            .output()
            .unwrap_or_else(|_| die("'go' não encontrado"));
        if !tidy.status.success() {
            eprintln!("{RED}erro:{RESET} {}", String::from_utf8_lossy(&tidy.stderr));
            process::exit(1);
        }
    }

    // copia .env do projeto para o dir temporário
    let env_src = cwd.join(".env");
    if env_src.exists() {
        let content = fs::read_to_string(&env_src).unwrap_or_default();
        write_file(&dir.join(".env"), &content);
    }

    step("migrações", &format!("{subcmd}..."));

    let status = Command::new("go")
        .args(["run", ".", subcmd])
        .current_dir(&dir)
        .env("HUSK_MIGRATIONS_DIR", migrations_dir.to_str().unwrap())
        .status()
        .unwrap_or_else(|_| die("'go' não encontrado"));

    if status.success() && subcmd != "status" {
        ok(&format!("migrate {subcmd} concluído"));
    } else if !status.success() {
        process::exit(status.code().unwrap_or(1));
    }
}

/// Converte Unix timestamp (segundos) em string YYYYMMDDHHMMSS.
/// Usa o algoritmo de Howard Hinnant para conversão de dias → data civil.
fn unix_to_timestamp(secs: u64) -> String {
    let sec = (secs % 60) as u32;
    let min = ((secs / 60) % 60) as u32;
    let hour = ((secs / 3600) % 24) as u32;
    let days = (secs / 86400) as i64;

    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let yr = if mo <= 2 { y + 1 } else { y };

    format!("{:04}{:02}{:02}{:02}{:02}{:02}", yr, mo, d, hour, min, sec)
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

    let husk_json = format!(
        r#"{{
    "name": "{name}",
    "dependencies": {{}}
}}
"#
    );
    fs::write(dir.join("husk.json"), husk_json).expect("falha ao criar husk.json");

    let gitignore = format!("{name}\n*.go\ngo.mod\ngo.sum\nvendor/\n");
    fs::write(dir.join(".gitignore"), gitignore).expect("falha ao criar .gitignore");

    ok(&format!("projeto {BOLD}{name}{RESET} criado"));
    eprintln!("{DIM}  cd {name} && husk run main.husk{RESET}");
}

// --- transpilação ---

fn transpile_file(file: &str) -> (String, StdlibDeps) {
    let step_start = Instant::now();
    step("transpilando", file);

    let mut source =
        fs::read_to_string(file).unwrap_or_else(|e| die(&format!("erro ao ler '{file}': {e}")));

    // auto-inclui .vendor.husk se existir
    let base_dir = Path::new(file).parent().unwrap_or(Path::new("."));
    let vendor_husk_path = base_dir.join(VENDOR_HUSK);
    if vendor_husk_path.exists() {
        let vendor_source = fs::read_to_string(&vendor_husk_path)
            .unwrap_or_else(|e| die(&format!("erro ao ler .vendor.husk: {e}")));
        source = vendor_source + &source;
    }

    let program = parse_source(&source, file);
    let mut visited = HashSet::new();
    let mut processing = HashSet::new();
    let merged = resolve_imports(program, base_dir, &mut visited, &mut processing, file);
    let stdlib = StdlibDeps::from_program(&merged);

    let mut codegen = Codegen::new();
    codegen.set_source_file(file);
    let go_code = codegen
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
    processing: &mut HashSet<PathBuf>,
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

        if !processing.insert(canonical.clone()) {
            die(&format!(
                "{current_file}: dependência circular detectada — '{}' já está sendo processado",
                imp.path
            ));
        }

        if visited.contains(&canonical) {
            processing.remove(&canonical);
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
            resolve_imports(mod_program, mod_dir, visited, processing, &canonical.to_string_lossy());

        processing.remove(&canonical);

        for item in mod_program.items {
            match item {
                Item::FnDef(f) => {
                    let mut f = f;
                    f.name = format!("{}_{}", imp.alias, f.name);
                    program.items.push(Item::FnDef(f));
                }
                Item::StructDef(_) => program.items.push(item),
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

    // Copia .env se existir no diretório de origem
    let source_dir = Path::new(husk_file).parent().unwrap_or(Path::new("."));
    let env_src = source_dir.join(".env");
    if env_src.exists() {
        let content =
            fs::read_to_string(&env_src).unwrap_or_else(|e| die(&format!("erro ao ler .env: {e}")));
        write_file(&dir.join(".env"), &content);
    }

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

/// Traduz erros do compilador Go (linhas em main.go) de volta para
/// as linhas do código .husk original usando os source maps embutidos.
fn translate_go_errors(stderr: &str, go_file: &Path, _husk_file: &str) -> String {
    // Lê o Go gerado e constrói mapa: linha_go → (arquivo_husk, linha_husk)
    let go_source = match fs::read_to_string(go_file) {
        Ok(s) => s,
        Err(_) => return stderr.to_string(),
    };

    // Mapa: linha Go (1-indexed) → (arquivo husk, linha husk)
    let mut line_map: std::collections::HashMap<usize, (String, usize)> =
        std::collections::HashMap::new();
    for (i, line) in go_source.lines().enumerate() {
        // procura // husk:arquivo:linha
        if let Some(rest) = line.trim().strip_prefix("// husk:") {
            if let Some(sep) = rest.rfind(':') {
                if let Ok(husk_line) = rest[sep + 1..].parse::<usize>() {
                    let husk_file_path = rest[..sep].to_string();
                    line_map.insert(i + 1, (husk_file_path, husk_line));
                }
            }
        }
    }

    if line_map.is_empty() {
        return stderr.to_string();
    }

    let mut result = String::new();
    for line in stderr.lines() {
        // Go errors: main.go:LINE:COL: mensagem
        if let Some(cap) = line.strip_prefix("main.go:") {
            let parts: Vec<&str> = cap.splitn(2, ':').collect();
            if parts.len() >= 2 {
                if let Ok(go_line) = parts[0].parse::<usize>() {
                    // Procura a anotação mais próxima antes ou na linha atual
                    let mut best: Option<(String, usize)> = None;
                    for (&gl, info) in &line_map {
                        if gl <= go_line {
                            best = Some((info.0.clone(), info.1));
                        }
                    }
                    if let Some((ref husk_path, husk_line)) = best {
                        let rest = parts[1..].join(":");
                        result.push_str(&format!(
                            "{RED}{BOLD}{}:{}{RESET} {}\n",
                            husk_path, husk_line, rest
                        ));
                        continue;
                    }
                }
            }
        }
        result.push_str(line);
        result.push('\n');
    }

    result
}
