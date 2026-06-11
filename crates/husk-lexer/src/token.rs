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
    Next,
    Import,
    As,
    Struct,
    Nil,

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

    // Fim de arquivo
    Eof,
}
