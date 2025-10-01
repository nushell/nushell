use crate::{Id, LanguageServer, path_to_uri, span_to_range, uri_to_path};
use lsp_textdocument::{FullTextDocument, TextDocuments};
use lsp_types::{
    DocumentSymbolParams, DocumentSymbolResponse, Location, Range, SymbolInformation, SymbolKind,
    Uri, WorkspaceSymbolParams, WorkspaceSymbolResponse,
};
use nu_protocol::{
    DeclId, ModuleId, Span, VarId,
    engine::{CachedFile, EngineState, StateWorkingSet},
};
use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, HashSet},
    hash::{Hash, Hasher},
};

/// Struct stored in cache, uri not included
#[derive(Clone, Debug, Eq, PartialEq)]
struct Symbol {
    name: String,
    kind: SymbolKind,
    range: Range,
}

impl Hash for Symbol {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.range.start.hash(state);
        self.range.end.hash(state);
    }
}

impl PartialOrd for Symbol {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Symbol {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.kind == other.kind {
            return self.range.start.cmp(&other.range.start);
        }
        match (self.kind, other.kind) {
            (SymbolKind::FUNCTION, _) => Ordering::Less,
            (_, SymbolKind::FUNCTION) => Ordering::Greater,
            _ => self.range.start.cmp(&other.range.start),
        }
    }
}

impl Symbol {
    fn to_symbol_information(&self, uri: &Uri) -> SymbolInformation {
        #[allow(deprecated)]
        SymbolInformation {
            location: Location {
                uri: uri.clone(),
                range: self.range,
            },
            name: self.name.to_owned(),
            kind: self.kind,
            container_name: None,
            deprecated: None,
            tags: None,
        }
    }
}

/// Cache symbols for each opened file to avoid repeated parsing
pub(crate) struct SymbolCache {
    /// Fuzzy matcher for symbol names
    matcher: Matcher,
    /// File Uri --> Symbols
    cache: BTreeMap<Uri, Vec<Symbol>>,
    /// If marked as dirty, parse on next request
    dirty_flags: BTreeMap<Uri, bool>,
}

impl SymbolCache {
    pub fn new() -> Self {
        SymbolCache {
            matcher: Matcher::new({
                let mut cfg = Config::DEFAULT;
                cfg.prefer_prefix = true;
                cfg
            }),
            cache: BTreeMap::new(),
            dirty_flags: BTreeMap::new(),
        }
    }

    pub fn mark_dirty(&mut self, uri: Uri, flag: bool) {
        self.dirty_flags.insert(uri, flag);
    }

    fn get_symbol_by_id(
        working_set: &StateWorkingSet,
        id: Id,
        doc: &FullTextDocument,
        doc_span: &Span,
    ) -> Option<Symbol> {
        match id {
            Id::Declaration(decl_id) => {
                let decl = working_set.get_decl(decl_id);
                let span = working_set.get_block(decl.block_id()?).span?;
                // multi-doc working_set, returns None if the Id is in other files
                if !doc_span.contains(span.start) {
                    return None;
                }
                Some(Symbol {
                    name: decl.name().to_string(),
                    kind: SymbolKind::FUNCTION,
                    range: span_to_range(&span, doc, doc_span.start),
                })
            }
            Id::Variable(var_id, _) => {
                let var = working_set.get_variable(var_id);
                let span = var.declaration_span;
                if !doc_span.contains(span.start) || span.end == span.start {
                    return None;
                }
                let range = span_to_range(&span, doc, doc_span.start);
                let name = doc.get_content(Some(range));
                Some(Symbol {
                    name: name.to_string(),
                    kind: SymbolKind::VARIABLE,
                    range,
                })
            }
            Id::Module(module_id, _) => {
                let module = working_set.get_module(module_id);
                let span = module.span?;
                if !doc_span.contains(span.start) {
                    return None;
                }
                Some(Symbol {
                    name: String::from_utf8(module.name()).ok()?,
                    kind: SymbolKind::MODULE,
                    range: span_to_range(&span, doc, doc_span.start),
                })
            }
            _ => None,
        }
    }

    fn extract_all_symbols(
        working_set: &StateWorkingSet,
        doc: &FullTextDocument,
        cached_file: &CachedFile,
    ) -> Vec<Symbol> {
        let mut all_symbols: Vec<Symbol> = (0..working_set.num_decls())
            .filter_map(|id| {
                Self::get_symbol_by_id(
                    working_set,
                    Id::Declaration(DeclId::new(id)),
                    doc,
                    &cached_file.covered_span,
                )
            })
            .chain((0..working_set.num_vars()).filter_map(|id| {
                Self::get_symbol_by_id(
                    working_set,
                    Id::Variable(VarId::new(id), [].into()),
                    doc,
                    &cached_file.covered_span,
                )
            }))
            .chain((0..working_set.num_modules()).filter_map(|id| {
                Self::get_symbol_by_id(
                    working_set,
                    Id::Module(ModuleId::new(id), [].into()),
                    doc,
                    &cached_file.covered_span,
                )
            }))
            // TODO: same variable symbol can be duplicated with different VarId
            .collect::<HashSet<Symbol>>()
            .into_iter()
            .collect();
        all_symbols.sort();
        all_symbols
    }

    /// Update the symbols of given uri if marked as dirty
    pub fn update(&mut self, uri: &Uri, engine_state: &EngineState, docs: &TextDocuments) {
        if *self.dirty_flags.get(uri).unwrap_or(&true) {
            let mut working_set = StateWorkingSet::new(engine_state);
            let content = docs
                .get_document_content(uri, None)
                .expect("Failed to get_document_content!")
                .as_bytes();
            nu_parser::parse(
                &mut working_set,
                Some(
                    uri_to_path(uri)
                        .to_str()
                        .expect("Failed to convert pathbuf to string"),
                ),
                content,
                false,
            );
            for cached_file in working_set.files() {
                let path = std::path::Path::new(&*cached_file.name);
                if !path.is_file() {
                    continue;
                }
                let target_uri = path_to_uri(path);
                let new_symbols = Self::extract_all_symbols(
                    &working_set,
                    docs.get_document(&target_uri)
                        .unwrap_or(&FullTextDocument::new(
                            "nu".to_string(),
                            0,
                            String::from_utf8((*cached_file.content).to_vec())
                                .expect("Invalid UTF-8"),
                        )),
                    cached_file,
                );
                self.cache.insert(target_uri.clone(), new_symbols);
                self.mark_dirty(target_uri, false);
            }
            self.mark_dirty(uri.clone(), false);
        };
    }

    pub fn drop(&mut self, uri: &Uri) {
        self.cache.remove(uri);
        self.dirty_flags.remove(uri);
    }

    pub fn update_all(&mut self, engine_state: &EngineState, docs: &TextDocuments) {
        for uri in docs.documents().keys() {
            self.update(uri, engine_state, docs);
        }
    }

    pub fn get_symbols_by_uri(&self, uri: &Uri) -> Option<Vec<SymbolInformation>> {
        Some(
            self.cache
                .get(uri)?
                .iter()
                .map(|s| s.to_symbol_information(uri))
                .collect(),
        )
    }

    pub fn get_fuzzy_matched_symbols(&mut self, query: &str) -> Vec<SymbolInformation> {
        let pat = Pattern::new(
            query,
            CaseMatching::Smart,
            Normalization::Smart,
            AtomKind::Fuzzy,
        );
        self.cache
            .iter()
            .flat_map(|(uri, symbols)| symbols.iter().map(|s| s.to_symbol_information(uri)))
            .filter_map(|s| {
                let mut buf = Vec::new();
                let name = Utf32Str::new(&s.name, &mut buf);
                pat.score(name, &mut self.matcher)?;
                Some(s)
            })
            .collect()
    }

    pub fn any_dirty(&self) -> bool {
        self.dirty_flags.values().any(|f| *f)
    }
}

impl LanguageServer {
    pub(crate) fn document_symbol(
        &mut self,
        params: &DocumentSymbolParams,
    ) -> Option<DocumentSymbolResponse> {
        let uri = &params.text_document.uri;
        let engine_state = self.new_engine_state(Some(uri));
        let docs = self.docs.lock().ok()?;
        self.symbol_cache.update(uri, &engine_state, &docs);
        self.symbol_cache
            .get_symbols_by_uri(uri)
            .map(DocumentSymbolResponse::Flat)
    }

    pub(crate) fn workspace_symbol(
        &mut self,
        params: &WorkspaceSymbolParams,
    ) -> Option<WorkspaceSymbolResponse> {
        if self.symbol_cache.any_dirty() {
            let engine_state = self.new_engine_state(None);
            let docs = self.docs.lock().ok()?;
            self.symbol_cache.update_all(&engine_state, &docs);
        }
        Some(WorkspaceSymbolResponse::Flat(
            self.symbol_cache
                .get_fuzzy_matched_symbols(params.query.as_str()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::path_to_uri;
    use crate::tests::{initialize_language_server, open_unchecked, result_from_message, update};
    use assert_json_diff::assert_json_eq;
    use lsp_server::{Connection, Message};
    use lsp_types::{
        DocumentSymbolParams, PartialResultParams, Position, Range, TextDocumentIdentifier, Uri,
        WorkDoneProgressParams, WorkspaceSymbolParams,
        request::{DocumentSymbolRequest, Request, WorkspaceSymbolRequest},
    };
    use nu_test_support::fs::fixtures;
    use rstest::rstest;

    fn create_position(line: u32, character: u32) -> serde_json::Value {
        serde_json::json!({ "line": line, "character": character })
    }

    fn create_range(
        start_line: u32,
        start_char: u32,
        end_line: u32,
        end_char: u32,
    ) -> serde_json::Value {
        serde_json::json!({
            "start": create_position(start_line, start_char),
            "end": create_position(end_line, end_char)
        })
    }

    fn create_symbol(
        name: &str,
        kind: u8,
        start_line: u32,
        start_char: u32,
        end_line: u32,
        end_char: u32,
    ) -> serde_json::Value {
        serde_json::json!({
            "name": name,
            "kind": kind,
            "location": {
                "range": create_range(start_line, start_char, end_line, end_char)
            }
        })
    }

    fn update_symbol_uri(symbols: &mut serde_json::Value, uri: &Uri) {
        if let Some(symbols_array) = symbols.as_array_mut() {
            for symbol in symbols_array {
                if let Some(location) = symbol.get_mut("location") {
                    location["uri"] = serde_json::json!(uri.to_string());
                }
            }
        }
    }

    fn send_document_symbol_request(client_connection: &Connection, uri: Uri) -> Message {
        client_connection
            .sender
            .send(Message::Request(lsp_server::Request {
                id: 1.into(),
                method: DocumentSymbolRequest::METHOD.to_string(),
                params: serde_json::to_value(DocumentSymbolParams {
                    text_document: TextDocumentIdentifier { uri },
                    partial_result_params: PartialResultParams::default(),
                    work_done_progress_params: WorkDoneProgressParams::default(),
                })
                .unwrap(),
            }))
            .unwrap();

        client_connection
            .receiver
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap()
    }

    fn send_workspace_symbol_request(client_connection: &Connection, query: String) -> Message {
        client_connection
            .sender
            .send(Message::Request(lsp_server::Request {
                id: 2.into(),
                method: WorkspaceSymbolRequest::METHOD.to_string(),
                params: serde_json::to_value(WorkspaceSymbolParams {
                    query,
                    partial_result_params: PartialResultParams::default(),
                    work_done_progress_params: WorkDoneProgressParams::default(),
                })
                .unwrap(),
            }))
            .unwrap();
        client_connection
            .receiver
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap()
    }

    #[rstest]
    #[case::special_variables(
        "span.nu",
        None,
        serde_json::json!([])
    )]
    #[case::basic_symbols(
        "foo.nu",
        None,
        serde_json::json!([
            create_symbol("def_foo", 12, 5, 15, 5, 20),
            create_symbol("var_foo", 13, 2, 4, 2, 11)
        ])
    )]
    #[case::after_update(
        "bar.nu",
        Some((String::default(), Range {
            start: Position { line: 2, character: 0 },
            end: Position { line: 4, character: 29 }
        })),
        serde_json::json!([
            create_symbol("var_bar", 13, 0, 13, 0, 20)
        ])
    )]
    fn document_symbol_request(
        #[case] filename: &str,
        #[case] update_op: Option<(String, Range)>,
        #[case] mut expected: serde_json::Value,
    ) {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp/symbols");
        script.push(filename);
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        if let Some((content, range)) = update_op {
            update(&client_connection, script.clone(), content, Some(range));
        }

        let resp = send_document_symbol_request(&client_connection, script.clone());

        // Update expected JSON to include the actual URI
        update_symbol_uri(&mut expected, &script);

        assert_json_eq!(result_from_message(resp), expected);
    }

    #[rstest]
    #[case::search_br(
        "br",
        serde_json::json!([
            create_symbol("def_bar", 12, 2, 22, 2, 27),
            create_symbol("var_bar", 13, 0, 13, 0, 20),
            create_symbol("module_bar", 2, 4, 26, 4, 27)
        ])
    )]
    #[case::search_foo(
        "foo",
        serde_json::json!([
            create_symbol("def_foo", 12, 5, 15, 5, 20),
            create_symbol("var_foo", 13, 2, 4, 2, 11)
        ])
    )]
    fn workspace_symbol_request(#[case] query: &str, #[case] mut expected: serde_json::Value) {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script_foo = fixtures();
        script_foo.push("lsp/symbols/foo.nu");
        let script_foo = path_to_uri(&script_foo);

        let mut script_bar = fixtures();
        script_bar.push("lsp/symbols/bar.nu");
        let script_bar = path_to_uri(&script_bar);

        open_unchecked(&client_connection, script_foo.clone());
        open_unchecked(&client_connection, script_bar.clone());
        let resp = send_workspace_symbol_request(&client_connection, query.to_string());

        // Determine which URI to use based on expected_file
        let target_uri = if query.contains("foo") {
            script_foo
        } else {
            script_bar
        };

        // Update expected JSON to include the actual URI
        update_symbol_uri(&mut expected, &target_uri);

        assert_json_eq!(result_from_message(resp), expected);
    }
}
