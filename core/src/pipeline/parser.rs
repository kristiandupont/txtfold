use super::tokenizer::Token;
use super::{AlgorithmDirective, ParseError, PathSegment, Stage, WhereOp, WhereValue};

pub(super) struct Parser {
    tokens: Vec<(Token, usize)>,
    pos: usize,
}

impl Parser {
    pub(super) fn new(tokens: Vec<(Token, usize)>) -> Self {
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
                    "where" => {
                        self.advance(); // consume 'where'
                        self.expect(&Token::LParen)?;
                        let field = self.parse_del_path_expr()?;
                        // Parse operator: ==, !=, or keyword (contains, starts_with, ends_with)
                        let op_pos = self.current_position();
                        let op = match self.advance() {
                            Some(Token::Eq) => WhereOp::Eq,
                            Some(Token::Ne) => WhereOp::Ne,
                            Some(Token::Ident(kw)) => match kw.as_str() {
                                "contains"    => WhereOp::Contains,
                                "starts_with" => WhereOp::StartsWith,
                                "ends_with"   => WhereOp::EndsWith,
                                other => {
                                    let msg = format!(
                                        "unknown where operator '{}'; expected ==, !=, \
                                         contains, starts_with, or ends_with", other
                                    );
                                    return Err(ParseError { position: op_pos, message: msg });
                                }
                            },
                            Some(tok) => {
                                let msg = format!(
                                    "expected operator (==, !=, contains, …), got {:?}", tok
                                );
                                return Err(ParseError { position: op_pos, message: msg });
                            }
                            None => return Err(ParseError {
                                position: op_pos,
                                message: "expected operator after field path in where()".to_string(),
                            }),
                        };
                        // Parse value: string literal or number
                        let val_pos = self.current_position();
                        let value = match self.advance() {
                            Some(Token::StringLit(s)) => WhereValue::String(s.clone()),
                            Some(Token::Float(f))     => WhereValue::Number(*f),
                            Some(Token::Integer(n))   => WhereValue::Number(*n as f64),
                            Some(tok) => {
                                let msg = format!(
                                    "expected string or number value in where(), got {:?}", tok
                                );
                                return Err(ParseError { position: val_pos, message: msg });
                            }
                            None => return Err(ParseError {
                                position: val_pos,
                                message: "expected value in where()".to_string(),
                            }),
                        };
                        self.expect(&Token::RParen)?;
                        Ok(Stage::Where { field, op, value })
                    }
                    other => Err(ParseError {
                        position: pos,
                        message: format!(
                            "unknown verb '{}' — valid verbs: del, where, group_by, label, top, \
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

    pub(super) fn parse_pipeline(mut self) -> Result<Vec<Stage>, ParseError> {
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
