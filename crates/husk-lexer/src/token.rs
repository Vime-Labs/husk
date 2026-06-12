#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Palavras-chave
    Fn,
    Return,
    Let,
    If,
    Else,
    Route,
    Middleware,
    Cors,
    Next,
    Import,
    As,
    Struct,
    Nil,
    For,
    In,
    Try,
    Catch,
    Retry,
    Break,
    Schema,
    Required,
    Model,

    // Métodos HTTP
    Get,
    Post,
    Put,
    Patch,
    Delete,

    // Tipos primitivos
    TyInt,
    TyString,
    TyBool,
    TyFloat,

    // Literais
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),

    // Identificadores
    Ident(String),

    // Símbolos
    LBrace,    // {
    RBrace,    // }
    LParen,    // (
    RParen,    // )
    LBracket,  // [
    RBracket,  // ]
    Comma,     // ,
    Colon,     // :
    Semicolon, // ;
    Dot,       // .
    DotDotDot, // ...
    Slash,     // /
    Bang,      // !
    Question,  // ?

    // Operadores
    Eq,      // =
    EqEq,    // ==
    NotEq,   // !=
    Lt,      // <
    Gt,      // >
    LtEq,    // <=
    GtEq,    // >=
    Plus,    // +
    Minus,   // -
    Star,    // *
    Percent, // %
    And,     // &&
    Or,      // ||

    // Comentários
    Comment(String),

    // Fim de arquivo
    Eof,
}

impl TokenKind {
    /// Se o token for uma palavra-chave (keyword, tipo, método HTTP),
    /// devolve o nome em string. Usado no parser para permitir keywords
    /// como chave em object literals (`{ model: "x" }`).
    pub fn keyword_name(&self) -> Option<&'static str> {
        match self {
            TokenKind::Fn => Some("fn"),
            TokenKind::Return => Some("return"),
            TokenKind::Let => Some("let"),
            TokenKind::If => Some("if"),
            TokenKind::Else => Some("else"),
            TokenKind::Route => Some("route"),
            TokenKind::Middleware => Some("middleware"),
            TokenKind::Cors => Some("cors"),
            TokenKind::Next => Some("next"),
            TokenKind::Import => Some("import"),
            TokenKind::As => Some("as"),
            TokenKind::Struct => Some("struct"),
            TokenKind::Nil => Some("nil"),
            TokenKind::For => Some("for"),
            TokenKind::In => Some("in"),
            TokenKind::Try => Some("try"),
            TokenKind::Catch => Some("catch"),
            TokenKind::Retry => Some("retry"),
            TokenKind::Break => Some("break"),
            TokenKind::Schema => Some("schema"),
            TokenKind::Required => Some("required"),
            TokenKind::Model => Some("model"),
            TokenKind::Get => Some("get"),
            TokenKind::Post => Some("post"),
            TokenKind::Put => Some("put"),
            TokenKind::Patch => Some("patch"),
            TokenKind::Delete => Some("delete"),
            TokenKind::TyInt => Some("int"),
            TokenKind::TyString => Some("string"),
            TokenKind::TyBool => Some("bool"),
            TokenKind::TyFloat => Some("float"),
            _ => None,
        }
    }
}
