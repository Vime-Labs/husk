use husk_lexer::Lexer;
use husk_parser::Parser;
use lsp_server::{Connection, Message, Notification, Request, Response};
use lsp_types::{
    Diagnostic, DiagnosticSeverity, InitializeParams, NumberOrString, Position,
    PublishDiagnosticsParams, Range, ServerCapabilities, TextDocumentSyncKind,
    TextDocumentSyncCapability, Url, notification::Notification as _, request::Request as _,
};
use std::collections::HashMap;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    let (connection, io_threads) = Connection::stdio();

    let capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::Full)),
        ..Default::default()
    };

    let server_capabilities = serde_json::to_value(&capabilities).unwrap();
    let initialization_params = connection.initialize(server_capabilities)?;
    let _params: InitializeParams = serde_json::from_value(initialization_params).unwrap();

    let mut state = ServerState {
        documents: HashMap::new(),
    };

    main_loop(&connection, &mut state)?;

    io_threads.join()?;
    Ok(())
}

struct ServerState {
    documents: HashMap<Url, String>,
}

fn main_loop(connection: &Connection, state: &mut ServerState) -> Result<(), Box<dyn Error + Sync + Send>> {
    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                handle_request(connection, state, req)?;
            }
            Message::Notification(not) => {
                handle_notification(connection, state, not)?;
            }
            Message::Response(_) => {}
        }
    }
    Ok(())
}

fn handle_request(
    connection: &Connection,
    state: &mut ServerState,
    req: Request,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    match req.method.as_str() {
        _ => {
            let resp = Response::new_err(
                req.id,
                lsp_server::ErrorCode::MethodNotFound as i32,
                "método não suportado".into(),
            );
            connection.sender.send(Message::Response(resp))?;
        }
    }
    Ok(())
}

fn handle_notification(
    connection: &Connection,
    state: &mut ServerState,
    not: Notification,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    match not.method.as_str() {
        "textDocument/didOpen" => {
            let params: lsp_types::DidOpenTextDocumentParams =
                serde_json::from_value(not.params)?;
            let uri = params.text_document.uri.clone();
            state.documents.insert(uri.clone(), params.text_document.text);
            publish_diagnostics(connection, state, &uri)?;
        }
        "textDocument/didChange" => {
            let params: lsp_types::DidChangeTextDocumentParams =
                serde_json::from_value(not.params)?;
            let uri = params.text_document.uri.clone();
            if let Some(change) = params.content_changes.into_iter().last() {
                state.documents.insert(uri.clone(), change.text);
            }
            publish_diagnostics(connection, state, &uri)?;
        }
        "textDocument/didClose" => {
            let params: lsp_types::DidCloseTextDocumentParams =
                serde_json::from_value(not.params)?;
            state.documents.remove(&params.text_document.uri);
        }
        _ => {}
    }
    Ok(())
}

fn publish_diagnostics(
    connection: &Connection,
    state: &ServerState,
    uri: &Url,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let source = match state.documents.get(uri) {
        Some(s) => s.clone(),
        None => return Ok(()),
    };

    let mut diagnostics = Vec::new();

    // lex
    match Lexer::new(&source).tokenize() {
        Ok(tokens) => {
            // parse
            match Parser::new(tokens).parse() {
                Ok(program) => {
                    // semantic analysis
                    let errors = husk_analyzer::analyze(&program);
                    for err in &errors {
                        diagnostics.push(Diagnostic {
                            range: Range {
                                start: Position {
                                    line: err.span.line.saturating_sub(1) as u32,
                                    character: err.span.col.saturating_sub(1) as u32,
                                },
                                end: Position {
                                    line: err.span.line.saturating_sub(1) as u32,
                                    character: (err.span.col + 10) as u32,
                                },
                            },
                            severity: Some(DiagnosticSeverity::ERROR),
                            message: err.message.clone(),
                            ..Default::default()
                        });
                    }
                }
                Err(e) => {
                    diagnostics.push(Diagnostic {
                        range: Range {
                            start: Position {
                                line: e.span.line.saturating_sub(1) as u32,
                                character: e.span.col.saturating_sub(1) as u32,
                            },
                            end: Position {
                                line: e.span.line.saturating_sub(1) as u32,
                                character: (e.span.col + 10) as u32,
                            },
                        },
                        severity: Some(DiagnosticSeverity::ERROR),
                        message: e.message,
                        ..Default::default()
                    });
                }
            }
        }
        Err(e) => {
            diagnostics.push(Diagnostic {
                range: Range {
                    start: Position {
                        line: e.span.line.saturating_sub(1) as u32,
                        character: e.span.col.saturating_sub(1) as u32,
                    },
                    end: Position {
                        line: e.span.line.saturating_sub(1) as u32,
                        character: (e.span.col + 10) as u32,
                    },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                message: e.message,
                ..Default::default()
            });
        }
    }

    let params = PublishDiagnosticsParams {
        uri: uri.clone(),
        diagnostics,
        version: None,
    };

    connection.sender.send(Message::Notification(Notification {
        method: "textDocument/publishDiagnostics".into(),
        params: serde_json::to_value(&params).unwrap(),
    }))?;

    Ok(())
}
