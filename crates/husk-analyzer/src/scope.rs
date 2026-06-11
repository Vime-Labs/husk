use husk_lexer::Span;
use husk_parser::ast::*;
use std::collections::HashMap;

/// Tipo resolvido — após análise semântica
#[derive(Debug, Clone, PartialEq)]
pub enum TypeInfo {
    Int,
    Float,
    String,
    Bool,
    Error,
    /// map[string]interface{}
    Map,
    /// []T
    List(Box<TypeInfo>),
    /// struct nome
    Struct(String),
    /// retorno de função que não pôde ser inferido
    Unknown,
}

impl TypeInfo {
    pub fn from_ast(ty: &Type) -> Self {
        match ty {
            Type::Int => TypeInfo::Int,
            Type::Float => TypeInfo::Float,
            Type::String => TypeInfo::String,
            Type::Bool => TypeInfo::Bool,
            Type::Error => TypeInfo::Error,
            Type::Map => TypeInfo::Map,
            Type::List(inner) => TypeInfo::List(Box::new(Self::from_ast(inner))),
            Type::Named(name) => TypeInfo::Struct(name.clone()),
        }
    }

    /// Tenta inferir o tipo de uma expressão literal
    pub fn from_lit(lit: &Lit) -> Self {
        match lit {
            Lit::Int(_) => TypeInfo::Int,
            Lit::Float(_) => TypeInfo::Float,
            Lit::Str(_) => TypeInfo::String,
            Lit::Bool(_) => TypeInfo::Bool,
        }
    }

    pub fn is_numeric(&self) -> bool {
        matches!(self, TypeInfo::Int | TypeInfo::Float)
    }

    pub fn name(&self) -> &'static str {
        match self {
            TypeInfo::Int => "int",
            TypeInfo::Float => "float",
            TypeInfo::String => "string",
            TypeInfo::Bool => "bool",
            TypeInfo::Error => "error",
            TypeInfo::Map => "map",
            TypeInfo::List(_) => "list",
            TypeInfo::Struct(n) => "(struct)",
            TypeInfo::Unknown => "unknown",
        }
    }
}

/// Símbolo — o que cada nome pode significar no escopo
#[derive(Debug, Clone)]
pub enum Symbol {
    Variable(TypeInfo),
    Function(FnSignature),
    Struct(Vec<StructField>),
    Middleware,
    /// alias de import (não precisa de tipo, o codegen resolve)
    Module,
}

#[derive(Debug, Clone)]
pub struct FnSignature {
    pub params: Vec<(String, TypeInfo)>,
    pub return_types: Vec<TypeInfo>,
}

/// Escopo aninhado (ex: dentro de um bloco if/else)
#[derive(Debug, Clone)]
pub struct Scope {
    symbols: HashMap<String, Symbol>,
    parent: Option<Box<Scope>>,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            parent: None,
        }
    }

    pub fn child(&self) -> Self {
        Self {
            symbols: HashMap::new(),
            parent: Some(Box::new(self.clone())),
        }
    }

    /// Declara um símbolo no escopo atual. Retorna erro se já existir.
    pub fn declare(
        &mut self,
        name: &str,
        symbol: Symbol,
        span: &Span,
    ) -> Result<(), SemanticError> {
        if self.symbols.contains_key(name) {
            return Err(SemanticError {
                message: format!("'{}' já foi declarado neste escopo", name),
                span: span.clone(),
            });
        }
        self.symbols.insert(name.to_string(), symbol);
        Ok(())
    }

    /// Busca um símbolo na cadeia de escopos
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.symbols
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.lookup(name)))
    }
}

/// Erro semântico
#[derive(Debug, Clone)]
pub struct SemanticError {
    pub message: String,
    pub span: Span,
}

impl SemanticError {
    pub fn new(msg: impl Into<String>, span: Span) -> Self {
        Self {
            message: msg.into(),
            span,
        }
    }
}
