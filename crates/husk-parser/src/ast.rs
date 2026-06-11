use husk_lexer::Span;

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub enum Item {
    FnDef(FnDef),
    RouteDef(RouteDef),
    StructDef(StructDef),
    Import(ImportDef),
    MiddlewareDef(MiddlewareDef),
}

/// fn nome(params) tipo_retorno { corpo }
#[derive(Debug, Clone)]
pub struct FnDef {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: ReturnType,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Type,
}

/// Tipo de retorno de uma função
#[derive(Debug, Clone)]
pub enum ReturnType {
    None,
    Single(Type),
    /// (Type, error) — padrão Go para funções que podem falhar
    Tuple(Vec<Type>),
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<StructField>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub struct ImportDef {
    pub path: String,
    pub alias: String,
    /// true para `import "husk/..."` — módulo da stdlib
    pub is_stdlib: bool,
    pub span: Span,
}

/// middleware nome { corpo }
#[derive(Debug, Clone)]
pub struct MiddlewareDef {
    pub name: String,
    pub body: Block,
    pub span: Span,
}

/// route MÉTODO /caminho [mw1, mw2] { corpo }
#[derive(Debug, Clone)]
pub struct RouteDef {
    pub method: HttpMethod,
    pub path: RoutePath,
    pub middlewares: Vec<String>,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

#[derive(Debug, Clone)]
pub struct RoutePath {
    pub segments: Vec<PathSegment>,
}

#[derive(Debug, Clone)]
pub enum PathSegment {
    Literal(String),
    Param(String),
}

impl RoutePath {
    pub fn to_string(&self) -> String {
        let mut s = String::new();
        for seg in &self.segments {
            s.push('/');
            match seg {
                PathSegment::Literal(lit) => s.push_str(lit),
                PathSegment::Param(name) => {
                    s.push('{');
                    s.push_str(name);
                    s.push('}');
                }
            }
        }
        if s.is_empty() {
            s.push('/');
        }
        s
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    /// return expr  ou  return expr, expr  (multi-retorno)
    Return(Vec<Expr>),
    Let(LetStmt),
    /// let a, b = expr  — multi-atribuição para funções que retornam (valor, error)
    LetMulti(LetMultiStmt),
    /// let x = expr?  — try com propagação de erro HTTP
    TryLet(TryLetStmt),
    If(IfStmt),
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub struct LetStmt {
    pub name: String,
    pub ty: Option<Type>,
    pub value: Expr,
}

#[derive(Debug, Clone)]
pub struct LetMultiStmt {
    pub names: Vec<String>,
    pub value: Expr,
}

/// let x = expr?  — try: desestrutura (valor, error) e propaga erro como resposta HTTP
#[derive(Debug, Clone)]
pub struct TryLetStmt {
    pub name: String,
    pub call: Expr,
    pub status_code: Option<i64>,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct IfStmt {
    pub condition: Expr,
    pub then_block: Block,
    pub else_block: Option<Block>,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Lit(Lit),
    Nil,
    Ident(String),
    Call(CallExpr),
    FieldAccess(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),
    BinOp(Box<Expr>, BinOp, Box<Expr>),
    Unary(UnaryOp, Box<Expr>),
    MapLit(MapLit),
    StructInit(StructInit),
}

#[derive(Debug, Clone)]
pub struct CallExpr {
    pub callee: Box<Expr>,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone)]
pub struct MapLit {
    pub fields: Vec<(String, Expr)>,
}

#[derive(Debug, Clone)]
pub struct StructInit {
    pub name: String,
    pub fields: Vec<(String, Expr)>,
}

#[derive(Debug, Clone)]
pub enum Lit {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
}

#[derive(Debug, Clone)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Not,
    Neg,
}

#[derive(Debug, Clone)]
pub enum Type {
    Int,
    Float,
    String,
    Bool,
    Error,
    /// map[string]interface{} — resultado de db.query_one e afins
    Map,
    /// []T — resultado de db.query e listas
    List(Box<Type>),
    Named(String),
}
