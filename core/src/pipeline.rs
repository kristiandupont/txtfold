//! Pipeline expression parser and executor.
//!
//! A pipeline is a `|`-separated sequence of stages, e.g.:
//!
//! ```text
//! .diagnostics[] | del(.sourceCode) | group_by(.category)
//! similar(0.8) | top(20)
//! outliers
//! ```
//!
//! # Stage taxonomy
//!
//! Pre-processing stages transform the input before the algorithm sees it:
//! - `PathSelect` — navigate into a JSON subtree (`.foo[]`, `.foo[0]`)
//! - `Del` — remove fields from each JSON object
//!
//! Algorithm stages select the analysis algorithm (at most one per pipeline,
//! must be the terminal verb or the only non-modifier verb):
//! - `GroupBy` — value-based frequency table
//! - `AlgorithmVerb` — one of: `summarize`, `similar(t)`, `patterns`, `outliers`,
//!   `schemas`, `subtree`
//!
//! Post-processing stages modify the output after the algorithm runs:
//! - `Top(n)` — keep the N largest groups; move the rest to outliers
//! - `Label(field)` — relabel groups using the value of a field
//!
//! # jaq boundary (future)
//! Pre-processing stages that return `Value` (path selection, del, future
//! `select`, `map`) are the natural domain of jaq. The `Stage` enum reserves a
//! `Jaq` variant so the handoff point is explicit in the type system without
//! requiring a rewrite when jaq is integrated.

use crate::entry::Entry;
use serde_json::Value;

// ── Public types ─────────────────────────────────────────────────────────────

/// A segment in a JSON path expression.
#[derive(Debug, Clone, PartialEq)]
pub enum PathSegment {
    /// `.field` — navigate into an object field.
    Field(String),
    /// `[]` or `[*]` — iterate all elements of an array.
    All,
    /// `[n]` — select element at index n.
    Index(usize),
}

/// A single stage in a pipeline.
#[derive(Debug, Clone, PartialEq)]
pub enum Stage {
    // ── Pre-processing ───────────────────────────────────────────────────────
    /// Navigate into a JSON subtree before analysis. JSON-only.
    PathSelect(Vec<PathSegment>),
    /// Remove fields (by dotted path) from each JSON object. JSON-only.
    /// Each inner `Vec<String>` is the sequence of field-name segments in the path,
    /// e.g. `del(.location.sourceCode)` → `vec![vec!["location", "sourceCode"]]`.
    Del(Vec<Vec<String>>),

    // ── Algorithm selection ──────────────────────────────────────────────────
    /// Value-based frequency table grouped by a field. JSON (and future line/block).
    GroupBy(String),
    /// One of the named algorithm verbs.
    AlgorithmVerb(AlgorithmDirective),

    // ── Post-processing ──────────────────────────────────────────────────────
    /// Keep the N largest groups; move the rest to a remainder bucket.
    Top(usize),
    /// Relabel each group using the value of a field.
    Label(String),

    /// Reserved for future jaq pre-processing integration.
    /// The parser never emits this variant today; it exists so the type system
    /// makes the jaq/txtfold boundary explicit when jaq is wired in.
    #[allow(dead_code)]
    Jaq(String),
}

/// Algorithm directive — the algorithm that should run.
#[derive(Debug, Clone, PartialEq)]
pub enum AlgorithmDirective {
    /// Default: fixed per-format table (json→subtree, line/block→template).
    Summarize,
    /// Edit-distance clustering at threshold `t`.
    Similar(f64),
    /// Template extraction algorithm.
    Patterns,
    /// N-gram outlier detection algorithm.
    Outliers,
    /// Schema clustering algorithm (JSON).
    Schemas,
    /// Subtree algorithm (JSON).
    Subtree,
}

/// Input handed to the pipeline executor.
#[derive(Debug, Clone)]
pub enum PipelineInput {
    Json(Vec<Value>),
    Text(Vec<Entry>),
}

/// Result returned by [`apply_pipeline`].
#[derive(Debug)]
pub struct PipelineResult {
    /// Transformed input after pre-processing stages.
    pub input: PipelineInput,
    /// Algorithm to run (from the terminal algorithm verb, or `Summarize`).
    pub algorithm: AlgorithmDirective,
    /// Optional value-based grouping field (from `group_by`).
    pub group_by_field: Option<String>,
    /// Truncate output to N groups after the algorithm runs.
    pub top: Option<usize>,
    /// Relabel groups by this field after the algorithm runs.
    pub label: Option<String>,
}

/// A parse error with a byte position and a human-readable hint.
#[derive(Debug, Clone)]
pub struct ParseError {
    /// Byte offset into the expression string where the problem was detected.
    pub position: usize,
    /// Human-readable description of the problem.
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "parse error at position {}: {}", self.position, self.message)
    }
}

// ── Tokenizer ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Dot,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Pipe,
    Star,
    Ident(String),
    Integer(usize),
    Float(f64),
}

struct Tokenizer<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Tokenizer<'a> {
    fn new(input: &'a str) -> Self {
        Tokenizer { input, pos: 0 }
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn advance_char(&mut self) {
        if let Some(c) = self.peek_char() {
            self.pos += c.len_utf8();
        }
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek_char(), Some(c) if c.is_ascii_whitespace()) {
            self.advance_char();
        }
    }

    /// Lex one token, returning `(token, start_pos)`.
    fn next_token(&mut self) -> Result<Option<(Token, usize)>, ParseError> {
        self.skip_whitespace();
        let start = self.pos;
        let Some(c) = self.peek_char() else {
            return Ok(None);
        };

        let tok = match c {
            '.' => { self.advance_char(); Token::Dot }
            '(' => { self.advance_char(); Token::LParen }
            ')' => { self.advance_char(); Token::RParen }
            '[' => { self.advance_char(); Token::LBracket }
            ']' => { self.advance_char(); Token::RBracket }
            ',' => { self.advance_char(); Token::Comma }
            '|' => { self.advance_char(); Token::Pipe }
            '*' => { self.advance_char(); Token::Star }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let mut s = String::new();
                while matches!(self.peek_char(), Some(ch) if ch.is_ascii_alphanumeric() || ch == '_') {
                    s.push(self.peek_char().unwrap());
                    self.advance_char();
                }
                Token::Ident(s)
            }
            c if c.is_ascii_digit() => {
                let mut s = String::new();
                while matches!(self.peek_char(), Some(ch) if ch.is_ascii_digit()) {
                    s.push(self.peek_char().unwrap());
                    self.advance_char();
                }
                if self.peek_char() == Some('.') {
                    // Could be a float — peek ahead.
                    let saved_pos = self.pos;
                    self.advance_char(); // consume '.'
                    if matches!(self.peek_char(), Some(ch) if ch.is_ascii_digit()) {
                        s.push('.');
                        while matches!(self.peek_char(), Some(ch) if ch.is_ascii_digit()) {
                            s.push(self.peek_char().unwrap());
                            self.advance_char();
                        }
                        let v: f64 = s.parse().map_err(|_| ParseError {
                            position: start,
                            message: format!("invalid float literal '{}'", s),
                        })?;
                        Token::Float(v)
                    } else {
                        // Wasn't a float, backtrack the dot.
                        self.pos = saved_pos;
                        let v: usize = s.parse().map_err(|_| ParseError {
                            position: start,
                            message: format!("invalid integer literal '{}'", s),
                        })?;
                        Token::Integer(v)
                    }
                } else {
                    let v: usize = s.parse().map_err(|_| ParseError {
                        position: start,
                        message: format!("invalid integer literal '{}'", s),
                    })?;
                    Token::Integer(v)
                }
            }
            other => {
                return Err(ParseError {
                    position: start,
                    message: format!("unexpected character '{}'", other),
                });
            }
        };

        Ok(Some((tok, start)))
    }

    fn tokenize(mut self) -> Result<Vec<(Token, usize)>, ParseError> {
        let mut tokens = Vec::new();
        while let Some(tok) = self.next_token()? {
            tokens.push(tok);
        }
        Ok(tokens)
    }
}

// ── Parser ────────────────────────────────────────────────────────────────────

struct Parser {
    tokens: Vec<(Token, usize)>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<(Token, usize)>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn current_position(&self) -> usize {
        self.tokens.get(self.pos).map(|(_, p)| *p).unwrap_or(usize::MAX)
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|(t, _)| t)
    }

    fn peek2(&self) -> Option<&Token> {
        self.tokens.get(self.pos + 1).map(|(t, _)| t)
    }

    fn advance(&mut self) -> Option<&Token> {
        let tok = self.tokens.get(self.pos).map(|(t, _)| t);
        if tok.is_some() { self.pos += 1; }
        tok
    }

    fn expect(&mut self, expected: &Token) -> Result<(), ParseError> {
        let pos = self.current_position();
        match self.advance() {
            Some(tok) if tok == expected => Ok(()),
            Some(tok) => {
                let msg = format!("expected {:?}, got {:?}", expected, tok);
                Err(ParseError { position: pos, message: msg })
            }
            None => Err(ParseError {
                position: pos,
                message: format!("expected {:?}, got end of input", expected),
            }),
        }
    }

    /// Parse `field_expr` = `"." ident` (single field name, for `label`).
    fn parse_field_expr(&mut self) -> Result<String, ParseError> {
        self.expect(&Token::Dot)?;
        let pos = self.current_position();
        match self.advance() {
            Some(Token::Ident(name)) => Ok(name.clone()),
            Some(tok) => {
                let msg = format!("expected field name after '.', got {:?}", tok);
                Err(ParseError { position: pos, message: msg })
            }
            None => Err(ParseError {
                position: pos,
                message: "expected field name after '.', got end of input".to_string(),
            }),
        }
    }

    /// Parse the argument to `group_by()`.
    ///
    /// Accepts either:
    /// - `.field`   — JSON object field (existing behaviour)
    /// - `slot[N]`  — Nth non-whitespace token of a line/block entry
    fn parse_group_by_arg(&mut self) -> Result<String, ParseError> {
        // Check for `slot[N]` — no leading dot, ident "slot" followed by `[`.
        if matches!(self.peek(), Some(Token::Ident(s)) if s == "slot") {
            if matches!(self.peek2(), Some(Token::LBracket)) {
                self.advance(); // consume 'slot'
                self.expect(&Token::LBracket)?;
                let pos = self.current_position();
                let n = match self.advance() {
                    Some(Token::Integer(n)) => *n,
                    Some(tok) => {
                        return Err(ParseError {
                            position: pos,
                            message: format!(
                                "expected integer index in slot[N], got {:?}", tok
                            ),
                        });
                    }
                    None => {
                        return Err(ParseError {
                            position: pos,
                            message: "expected integer index in slot[N], got end of input"
                                .to_string(),
                        });
                    }
                };
                self.expect(&Token::RBracket)?;
                return Ok(format!("slot[{}]", n));
            }
        }
        // Fall back to `.field` syntax for JSON.
        self.parse_field_expr()
    }

    /// Parse a dotted field path for use in `del()`:
    /// `"." ident ("." ident)*` → `Vec<String>` of path segments.
    ///
    /// Examples: `.sourceCode` → `["sourceCode"]`,
    ///           `.location.sourceCode` → `["location", "sourceCode"]`.
    fn parse_del_path_expr(&mut self) -> Result<Vec<String>, ParseError> {
        self.expect(&Token::Dot)?;
        let pos = self.current_position();
        let first = match self.advance() {
            Some(Token::Ident(name)) => name.clone(),
            Some(tok) => {
                let msg = format!("expected field name after '.', got {:?}", tok);
                return Err(ParseError { position: pos, message: msg });
            }
            None => {
                return Err(ParseError {
                    position: pos,
                    message: "expected field name after '.', got end of input".to_string(),
                });
            }
        };

        let mut segments = vec![first];
        // Consume additional ".ident" segments.
        while self.peek() == Some(&Token::Dot)
            && matches!(self.peek2(), Some(Token::Ident(_)))
        {
            self.advance(); // consume '.'
            if let Some(Token::Ident(name)) = self.advance() {
                segments.push(name.clone());
            }
        }
        Ok(segments)
    }

    /// Parse `del_field_list` = `del_path_expr ("," del_path_expr)*`
    fn parse_del_field_list(&mut self) -> Result<Vec<Vec<String>>, ParseError> {
        let mut fields = vec![self.parse_del_path_expr()?];
        while self.peek() == Some(&Token::Comma) {
            self.advance(); // consume ','
            fields.push(self.parse_del_path_expr()?);
        }
        Ok(fields)
    }

    /// Parse a path expression starting from the first `.`.
    ///
    /// Grammar: `"." ident ( "[" ("*" | integer | "") "]" )* ("." ident)*`
    ///
    /// The leading dot has already been peeked but not consumed.
    fn parse_path_expr(&mut self) -> Result<Stage, ParseError> {
        self.advance(); // consume leading '.'

        let mut segments = Vec::new();

        // First field name
        match self.peek() {
            Some(Token::Ident(_)) => {
                if let Some(Token::Ident(name)) = self.advance() {
                    segments.push(PathSegment::Field(name.clone()));
                }
            }
            _ => {
                // Bare `.` — refers to the current value. Not meaningful as a
                // standalone path select; callers should handle this as field_expr.
                return Err(ParseError {
                    position: self.current_position(),
                    message: "expected field name after '.'; use 'del(.field)' or a verb".to_string(),
                });
            }
        }

        // Optional bracket and further field accesses
        loop {
            match self.peek() {
                Some(Token::LBracket) => {
                    self.advance(); // consume '['
                    match self.peek() {
                        Some(Token::RBracket) => {
                            self.advance(); // consume ']'
                            segments.push(PathSegment::All);
                        }
                        Some(Token::Star) => {
                            self.advance(); // consume '*'
                            self.expect(&Token::RBracket)?;
                            segments.push(PathSegment::All);
                        }
                        Some(Token::Integer(_)) => {
                            if let Some(Token::Integer(n)) = self.advance() {
                                let n = *n;
                                self.expect(&Token::RBracket)?;
                                segments.push(PathSegment::Index(n));
                            }
                        }
                        Some(tok) => {
                            return Err(ParseError {
                                position: self.current_position(),
                                message: format!(
                                    "expected ']', '*', or integer in bracket, got {:?}", tok
                                ),
                            });
                        }
                        None => {
                            return Err(ParseError {
                                position: self.current_position(),
                                message: "unexpected end of input inside '['".to_string(),
                            });
                        }
                    }
                }
                Some(Token::Dot) if matches!(self.peek2(), Some(Token::Ident(_))) => {
                    self.advance(); // consume '.'
                    if let Some(Token::Ident(name)) = self.advance() {
                        segments.push(PathSegment::Field(name.clone()));
                    }
                }
                _ => break,
            }
        }

        Ok(Stage::PathSelect(segments))
    }

    /// Parse one stage (path expression or verb).
    fn parse_stage(&mut self) -> Result<Stage, ParseError> {
        let pos = self.current_position();
        match self.peek() {
            Some(Token::Dot) => {
                // Could be a path expression. Peek ahead to decide: if the
                // next-next token is an ident, it's a path expression.
                // Standalone `.` is not a valid stage here.
                match self.peek2() {
                    Some(Token::Ident(_)) => self.parse_path_expr(),
                    _ => Err(ParseError {
                        position: pos,
                        message: "standalone '.' is not a valid stage; \
                                  did you mean '.field[]'?".to_string(),
                    }),
                }
            }
            Some(Token::Ident(name)) => {
                let name = name.clone();
                match name.as_str() {
                    "del" => {
                        self.advance(); // consume 'del'
                        self.expect(&Token::LParen)?;
                        let fields = self.parse_del_field_list()?;
                        self.expect(&Token::RParen)?;
                        Ok(Stage::Del(fields))
                    }
                    "group_by" => {
                        self.advance(); // consume 'group_by'
                        self.expect(&Token::LParen)?;
                        let field = self.parse_group_by_arg()?;
                        self.expect(&Token::RParen)?;
                        Ok(Stage::GroupBy(field))
                    }
                    "label" => {
                        self.advance(); // consume 'label'
                        self.expect(&Token::LParen)?;
                        let field = self.parse_field_expr()?;
                        self.expect(&Token::RParen)?;
                        Ok(Stage::Label(field))
                    }
                    "top" => {
                        self.advance(); // consume 'top'
                        self.expect(&Token::LParen)?;
                        let pos = self.current_position();
                        let n = match self.advance() {
                            Some(Token::Integer(n)) => *n,
                            Some(tok) => {
                                let msg = format!("top() requires an integer argument, got {:?}", tok);
                                return Err(ParseError { position: pos, message: msg });
                            }
                            None => return Err(ParseError {
                                position: pos,
                                message: "top() requires an integer argument".to_string(),
                            }),
                        };
                        self.expect(&Token::RParen)?;
                        Ok(Stage::Top(n))
                    }
                    "similar" => {
                        self.advance(); // consume 'similar'
                        self.expect(&Token::LParen)?;
                        let pos = self.current_position();
                        let threshold = match self.advance() {
                            Some(Token::Float(f)) => *f,
                            Some(Token::Integer(n)) => *n as f64,
                            Some(tok) => {
                                let msg = format!(
                                    "similar() requires a float threshold, got {:?}", tok
                                );
                                return Err(ParseError { position: pos, message: msg });
                            }
                            None => {
                                return Err(ParseError {
                                    position: pos,
                                    message: "similar() requires a float threshold".to_string(),
                                });
                            }
                        };
                        self.expect(&Token::RParen)?;
                        Ok(Stage::AlgorithmVerb(AlgorithmDirective::Similar(threshold)))
                    }
                    "summarize" => {
                        self.advance();
                        Ok(Stage::AlgorithmVerb(AlgorithmDirective::Summarize))
                    }
                    "patterns" => {
                        self.advance();
                        Ok(Stage::AlgorithmVerb(AlgorithmDirective::Patterns))
                    }
                    "outliers" => {
                        self.advance();
                        Ok(Stage::AlgorithmVerb(AlgorithmDirective::Outliers))
                    }
                    "schemas" => {
                        self.advance();
                        Ok(Stage::AlgorithmVerb(AlgorithmDirective::Schemas))
                    }
                    "subtree" => {
                        self.advance();
                        Ok(Stage::AlgorithmVerb(AlgorithmDirective::Subtree))
                    }
                    other => Err(ParseError {
                        position: pos,
                        message: format!(
                            "unknown verb '{}' — valid verbs: del, group_by, label, top, \
                             summarize, similar, patterns, outliers, schemas, subtree",
                            other
                        ),
                    }),
                }
            }
            Some(tok) => Err(ParseError {
                position: pos,
                message: format!("unexpected token {:?}; expected a path expression or verb", tok),
            }),
            None => Err(ParseError {
                position: pos,
                message: "unexpected end of pipeline".to_string(),
            }),
        }
    }

    fn parse_pipeline(mut self) -> Result<Vec<Stage>, ParseError> {
        let mut stages = Vec::new();
        stages.push(self.parse_stage()?);

        while self.peek() == Some(&Token::Pipe) {
            self.advance(); // consume '|'
            stages.push(self.parse_stage()?);
        }

        if self.pos < self.tokens.len() {
            return Err(ParseError {
                position: self.current_position(),
                message: format!(
                    "unexpected token {:?} after pipeline end",
                    self.tokens[self.pos].0
                ),
            });
        }

        Ok(stages)
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Parse a pipeline expression string into a list of stages.
///
/// Returns a [`ParseError`] with the byte offset and a hint on failure.
pub fn parse_pipeline(expr: &str) -> Result<Vec<Stage>, ParseError> {
    let tokens = Tokenizer::new(expr).tokenize()?;
    if tokens.is_empty() {
        return Err(ParseError {
            position: 0,
            message: "empty pipeline expression".to_string(),
        });
    }
    Parser::new(tokens).parse_pipeline()
}

/// Execute the pre-processing stages of a pipeline and extract the algorithm
/// directive and post-processing modifiers.
///
/// # Stage execution order
/// 1. Pre-processing stages (`PathSelect`, `Del`) are applied sequentially to
///    the input, transforming it before the algorithm sees it.
/// 2. The algorithm directive comes from the last `AlgorithmVerb` or `GroupBy`
///    stage (or `Summarize` if none is present).
/// 3. Post-processing modifiers (`Top`, `Label`) are collected and returned for
///    the caller to apply after the algorithm runs.
///
/// Returns `Err` if a JSON-only stage is used with text input.
pub fn apply_pipeline(
    stages: &[Stage],
    input: PipelineInput,
) -> Result<PipelineResult, String> {
    let mut input = input;
    let mut algorithm = AlgorithmDirective::Summarize;
    let mut group_by_field: Option<String> = None;
    let mut top: Option<usize> = None;
    let mut label: Option<String> = None;

    for stage in stages {
        match stage {
            Stage::PathSelect(segments) => {
                input = apply_path_select(input, segments)?;
            }
            Stage::Del(fields) => {
                input = apply_del(input, fields)?;
            }
            Stage::GroupBy(field) => {
                group_by_field = Some(field.clone());
                // GroupBy also drives algorithm selection.
                algorithm = AlgorithmDirective::Summarize; // placeholder; group_by_field signals the real path
            }
            Stage::AlgorithmVerb(dir) => {
                algorithm = dir.clone();
            }
            Stage::Top(n) => {
                top = Some(*n);
            }
            Stage::Label(field) => {
                label = Some(field.clone());
            }
            Stage::Jaq(_) => {
                return Err("jaq integration is not yet implemented".to_string());
            }
        }
    }

    Ok(PipelineResult {
        input,
        algorithm,
        group_by_field,
        top,
        label,
    })
}

// ── Pre-processing helpers ────────────────────────────────────────────────────

fn apply_path_select(input: PipelineInput, segments: &[PathSegment]) -> Result<PipelineInput, String> {
    match input {
        PipelineInput::Json(values) => {
            // Navigate each value through the path segments.
            let mut current: Vec<Value> = values;

            for seg in segments {
                current = match seg {
                    PathSegment::Field(name) => {
                        current
                            .into_iter()
                            .filter_map(|v| {
                                if let Value::Object(map) = v {
                                    map.get(name).cloned()
                                } else {
                                    None
                                }
                            })
                            .collect()
                    }
                    PathSegment::All => {
                        current
                            .into_iter()
                            .flat_map(|v| match v {
                                Value::Array(arr) => arr,
                                _ => vec![],
                            })
                            .collect()
                    }
                    PathSegment::Index(n) => {
                        current
                            .into_iter()
                            .filter_map(|v| {
                                if let Value::Array(arr) = v {
                                    arr.into_iter().nth(*n)
                                } else {
                                    None
                                }
                            })
                            .collect()
                    }
                };
            }

            Ok(PipelineInput::Json(current))
        }
        PipelineInput::Text(_) => Err(
            "path selection (e.g. '.foo[]') is only valid for JSON input; \
             use --format json or omit path stages for line/block input".to_string(),
        ),
    }
}

fn apply_del(input: PipelineInput, paths: &[Vec<String>]) -> Result<PipelineInput, String> {
    match input {
        PipelineInput::Json(values) => {
            let result = values
                .into_iter()
                .map(|v| {
                    let mut v = v;
                    for path in paths {
                        v = remove_at_path(v, path);
                    }
                    v
                })
                .collect();
            Ok(PipelineInput::Json(result))
        }
        PipelineInput::Text(_) => Err("del() is only valid for JSON input".to_string()),
    }
}

/// Recursively remove the field at `path` from `value`.
///
/// - Single-segment path: removes the key directly from the object.
/// - Multi-segment path: traverses nested objects and removes the terminal key.
/// - If any intermediate key is missing or the value is not an object: silently skip
///   (same behaviour as jq `del`).
fn remove_at_path(value: Value, path: &[String]) -> Value {
    if path.is_empty() {
        return value;
    }
    match value {
        Value::Object(mut map) => {
            if path.len() == 1 {
                map.remove(&path[0]);
            } else if let Some(nested) = map.remove(&path[0]) {
                let updated = remove_at_path(nested, &path[1..]);
                map.insert(path[0].clone(), updated);
            }
            // If the key doesn't exist, silently skip.
            Value::Object(map)
        }
        // Not an object at this level — silently skip.
        other => other,
    }
}

// ── Value-based group_by implementation ──────────────────────────────────────

/// Partition a list of JSON values by the string value of `field`.
///
/// Returns `(groups, ungrouped)` where `groups` is a list of `(field_value,
/// entries)` pairs sorted by descending count, and `ungrouped` contains values
/// that did not have the field (or had a non-string/non-scalar value).
pub fn partition_by_field(
    values: &[Value],
    field: &str,
) -> (Vec<(String, Vec<usize>)>, Vec<usize>) {
    use std::collections::HashMap;

    let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
    let mut ungrouped: Vec<usize> = Vec::new();

    for (idx, value) in values.iter().enumerate() {
        let key = match value {
            Value::Object(map) => map.get(field).and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                Value::Number(n) => Some(n.to_string()),
                Value::Bool(b) => Some(b.to_string()),
                Value::Null => Some("null".to_string()),
                _ => None,
            }),
            _ => None,
        };

        match key {
            Some(k) => groups.entry(k).or_default().push(idx),
            None => ungrouped.push(idx),
        }
    }

    let mut sorted: Vec<(String, Vec<usize>)> = groups.into_iter().collect();
    sorted.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
    (sorted, ungrouped)
}

// ── Known verb names (for CLI disambiguation) ─────────────────────────────────

/// Returns true if `s` is a pipeline verb name (used for CLI disambiguation).
pub fn is_verb_name(s: &str) -> bool {
    matches!(
        s,
        "summarize" | "similar" | "patterns" | "outliers"
            | "schemas" | "subtree" | "del" | "group_by"
            | "label" | "top"
    )
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_verb() {
        let stages = parse_pipeline("outliers").unwrap();
        assert_eq!(stages.len(), 1);
        assert_eq!(stages[0], Stage::AlgorithmVerb(AlgorithmDirective::Outliers));
    }

    #[test]
    fn test_parse_summarize() {
        let stages = parse_pipeline("summarize").unwrap();
        assert_eq!(stages[0], Stage::AlgorithmVerb(AlgorithmDirective::Summarize));
    }

    #[test]
    fn test_parse_similar() {
        let stages = parse_pipeline("similar(0.8)").unwrap();
        assert_eq!(stages[0], Stage::AlgorithmVerb(AlgorithmDirective::Similar(0.8)));
    }

    #[test]
    fn test_parse_similar_integer_threshold() {
        let stages = parse_pipeline("similar(1)").unwrap();
        assert_eq!(stages[0], Stage::AlgorithmVerb(AlgorithmDirective::Similar(1.0)));
    }

    #[test]
    fn test_parse_del() {
        let stages = parse_pipeline("del(.sourceCode, .dictionary)").unwrap();
        assert_eq!(
            stages[0],
            Stage::Del(vec![
                vec!["sourceCode".to_string()],
                vec!["dictionary".to_string()],
            ])
        );
    }

    #[test]
    fn test_parse_del_dotted_path() {
        let stages = parse_pipeline("del(.location.sourceCode)").unwrap();
        assert_eq!(
            stages[0],
            Stage::Del(vec![vec!["location".to_string(), "sourceCode".to_string()]])
        );
    }

    #[test]
    fn test_parse_del_mixed_paths() {
        let stages = parse_pipeline("del(.sourceCode, .location.file, .advices)").unwrap();
        assert_eq!(
            stages[0],
            Stage::Del(vec![
                vec!["sourceCode".to_string()],
                vec!["location".to_string(), "file".to_string()],
                vec!["advices".to_string()],
            ])
        );
    }

    #[test]
    fn test_parse_group_by() {
        let stages = parse_pipeline("group_by(.category)").unwrap();
        assert_eq!(stages[0], Stage::GroupBy("category".to_string()));
    }

    #[test]
    fn test_parse_top() {
        let stages = parse_pipeline("top(20)").unwrap();
        assert_eq!(stages[0], Stage::Top(20));
    }

    #[test]
    fn test_parse_label() {
        let stages = parse_pipeline("label(.name)").unwrap();
        assert_eq!(stages[0], Stage::Label("name".to_string()));
    }

    #[test]
    fn test_parse_path_select_all() {
        let stages = parse_pipeline(".diagnostics[]").unwrap();
        assert_eq!(
            stages[0],
            Stage::PathSelect(vec![
                PathSegment::Field("diagnostics".to_string()),
                PathSegment::All,
            ])
        );
    }

    #[test]
    fn test_parse_path_select_star() {
        let stages = parse_pipeline(".diagnostics[*]").unwrap();
        assert_eq!(
            stages[0],
            Stage::PathSelect(vec![
                PathSegment::Field("diagnostics".to_string()),
                PathSegment::All,
            ])
        );
    }

    #[test]
    fn test_parse_path_select_index() {
        let stages = parse_pipeline(".items[0]").unwrap();
        assert_eq!(
            stages[0],
            Stage::PathSelect(vec![
                PathSegment::Field("items".to_string()),
                PathSegment::Index(0),
            ])
        );
    }

    #[test]
    fn test_parse_multi_stage_pipeline() {
        let stages = parse_pipeline(".diagnostics[] | del(.sourceCode) | group_by(.category)").unwrap();
        assert_eq!(stages.len(), 3);
        assert_eq!(
            stages[0],
            Stage::PathSelect(vec![
                PathSegment::Field("diagnostics".to_string()),
                PathSegment::All,
            ])
        );
        assert_eq!(stages[1], Stage::Del(vec![vec!["sourceCode".to_string()]]));
        assert_eq!(stages[2], Stage::GroupBy("category".to_string()));
    }

    #[test]
    fn test_parse_similar_top_pipeline() {
        let stages = parse_pipeline("similar(0.8) | top(20)").unwrap();
        assert_eq!(stages[0], Stage::AlgorithmVerb(AlgorithmDirective::Similar(0.8)));
        assert_eq!(stages[1], Stage::Top(20));
    }

    #[test]
    fn test_parse_error_unknown_verb() {
        let err = parse_pipeline("frobnicate").unwrap_err();
        assert!(err.message.contains("frobnicate"));
    }

    #[test]
    fn test_parse_error_empty() {
        assert!(parse_pipeline("").is_err());
    }

    #[test]
    fn test_apply_pipeline_del() {
        let values = vec![serde_json::json!({"a": 1, "b": 2, "c": 3})];
        let stages = parse_pipeline("del(.b)").unwrap();
        let result = apply_pipeline(&stages, PipelineInput::Json(values)).unwrap();
        match result.input {
            PipelineInput::Json(vals) => {
                assert!(vals[0].get("a").is_some());
                assert!(vals[0].get("b").is_none());
                assert!(vals[0].get("c").is_some());
            }
            _ => panic!("expected JSON output"),
        }
    }

    #[test]
    fn test_apply_pipeline_del_dotted_path() {
        let values = vec![serde_json::json!({
            "category": "error",
            "location": {"file": "main.rs", "line": 42}
        })];
        let stages = parse_pipeline("del(.location.file)").unwrap();
        let result = apply_pipeline(&stages, PipelineInput::Json(values)).unwrap();
        match result.input {
            PipelineInput::Json(vals) => {
                // "category" untouched
                assert_eq!(vals[0]["category"], "error");
                // "location" still exists
                assert!(vals[0].get("location").is_some());
                // "location.file" removed
                assert!(vals[0]["location"].get("file").is_none());
                // "location.line" untouched
                assert_eq!(vals[0]["location"]["line"], 42);
            }
            _ => panic!("expected JSON output"),
        }
    }

    #[test]
    fn test_apply_pipeline_del_missing_intermediate_key() {
        // del(.a.b) where .a doesn't exist — should silently skip
        let values = vec![serde_json::json!({"x": 1})];
        let stages = parse_pipeline("del(.a.b)").unwrap();
        let result = apply_pipeline(&stages, PipelineInput::Json(values)).unwrap();
        match result.input {
            PipelineInput::Json(vals) => {
                assert_eq!(vals[0]["x"], 1);
            }
            _ => panic!("expected JSON output"),
        }
    }

    #[test]
    fn test_apply_pipeline_path_select() {
        let values = vec![serde_json::json!({"items": [{"x": 1}, {"x": 2}]})];
        let stages = parse_pipeline(".items[]").unwrap();
        let result = apply_pipeline(&stages, PipelineInput::Json(values)).unwrap();
        match result.input {
            PipelineInput::Json(vals) => {
                assert_eq!(vals.len(), 2);
                assert_eq!(vals[0]["x"], 1);
                assert_eq!(vals[1]["x"], 2);
            }
            _ => panic!("expected JSON output"),
        }
    }

    #[test]
    fn test_apply_pipeline_extracts_algorithm() {
        let stages = parse_pipeline("del(.x) | schemas").unwrap();
        let values = vec![serde_json::json!({"a": 1, "x": 9})];
        let result = apply_pipeline(&stages, PipelineInput::Json(values)).unwrap();
        assert_eq!(result.algorithm, AlgorithmDirective::Schemas);
        assert_eq!(result.top, None);
    }

    #[test]
    fn test_apply_pipeline_extracts_top_and_label() {
        let stages = parse_pipeline("patterns | top(5) | label(.name)").unwrap();
        let entries: Vec<Entry> = vec![];
        let result = apply_pipeline(&stages, PipelineInput::Text(entries)).unwrap();
        assert_eq!(result.algorithm, AlgorithmDirective::Patterns);
        assert_eq!(result.top, Some(5));
        assert_eq!(result.label, Some("name".to_string()));
    }

    #[test]
    fn test_del_on_text_input_errors() {
        let stages = parse_pipeline("del(.x)").unwrap();
        let entries: Vec<Entry> = vec![];
        let err = apply_pipeline(&stages, PipelineInput::Text(entries)).unwrap_err();
        assert!(err.contains("JSON"));
    }

    #[test]
    fn test_partition_by_field() {
        let values = vec![
            serde_json::json!({"level": "error", "msg": "a"}),
            serde_json::json!({"level": "warn",  "msg": "b"}),
            serde_json::json!({"level": "error", "msg": "c"}),
            serde_json::json!({"msg": "no level"}),
        ];
        let (groups, ungrouped) = partition_by_field(&values, "level");
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].0, "error"); // sorted by count desc
        assert_eq!(groups[0].1.len(), 2);
        assert_eq!(groups[1].0, "warn");
        assert_eq!(ungrouped.len(), 1);
    }

    #[test]
    fn test_parse_group_by_slot() {
        let stages = parse_pipeline("group_by(slot[3])").unwrap();
        assert_eq!(stages[0], Stage::GroupBy("slot[3]".to_string()));
    }

    #[test]
    fn test_parse_group_by_slot_zero() {
        let stages = parse_pipeline("group_by(slot[0])").unwrap();
        assert_eq!(stages[0], Stage::GroupBy("slot[0]".to_string()));
    }

    #[test]
    fn test_parse_group_by_slot_in_pipeline() {
        let stages = parse_pipeline("group_by(slot[2]) | top(10)").unwrap();
        assert_eq!(stages[0], Stage::GroupBy("slot[2]".to_string()));
        assert_eq!(stages[1], Stage::Top(10));
    }

    #[test]
    fn test_is_verb_name() {
        assert!(is_verb_name("summarize"));
        assert!(is_verb_name("outliers"));
        assert!(is_verb_name("group_by"));
        assert!(!is_verb_name("foo"));
        assert!(!is_verb_name("app.log"));
    }
}
