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
    CorsDef(CorsDef),
}

/// cors { origins: [...] methods: [...] headers: [...] }
#[derive(Debug, Clone)]
pub struct CorsDef {
    pub origins: Vec<String>,
    pub methods: Vec<String>,
    pub headers: Vec<String>,
    pub span: Span,
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

/// middleware nome [-> ctx_var] { corpo }
#[derive(Debug, Clone)]
pub struct MiddlewareDef {
    pub name: String,
    pub ctx_var: Option<String>,
    pub body: Block,
    pub span: Span,
}

/// route MÉTODO /caminho [mw1, mw2] [-> ctx_var] { corpo }
#[derive(Debug, Clone)]
pub struct RouteDef {
    pub method: HttpMethod,
    pub path: RoutePath,
    pub middlewares: Vec<String>,
    pub ctx_var: Option<String>,
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
    /// name, optional type (e.g. :id<int>)
    Param(String, Option<Type>),
}

impl RoutePath {
    pub fn to_string(&self) -> String {
        let mut s = String::new();
        for seg in &self.segments {
            s.push('/');
            match seg {
                PathSegment::Literal(lit) => s.push_str(lit),
                PathSegment::Param(name, _ty) => {
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

    /// Retorna o tipo associado a um parâmetro pelo nome, se houver
    pub fn param_type(&self, name: &str) -> Option<&Type> {
        for seg in &self.segments {
            if let PathSegment::Param(n, ty) = seg {
                if n == name {
                    return ty.as_ref();
                }
            }
        }
        None
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
    /// for item in expr { body }
    ForIn(ForInStmt),
    /// assignment: target = expr  (e.g. ctx.field = value)
    Assign(AssignStmt),
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

/// expr? [status] ["msg"]  — try operator em expressões
#[derive(Debug, Clone)]
pub struct TryExpr {
    pub expr: Box<Expr>,
    pub status_code: Option<i64>,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct IfStmt {
    pub condition: Expr,
    pub then_block: Block,
    pub else_block: Option<Block>,
}

    /// for item in expr { body }
    #[derive(Debug, Clone)]
    pub struct ForInStmt {
        pub item: String,
        pub collection: Expr,
        pub body: Block,
    }

    /// target = value — usado para ctx.field = expr
    #[derive(Debug, Clone)]
    pub struct AssignStmt {
        /// O alvo da atribuição (ex: ctx.field acessado como FieldAccess(Ident("ctx"), "field"))
        pub target: Box<Expr>,
        pub value: Expr,
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
    /// expr? [status] ["msg"]  — try operator
    Try(TryExpr),
    /// expr...  — spread: desestrutura map/array em argumentos
    Spread(Box<Expr>),
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
