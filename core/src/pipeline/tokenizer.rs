use super::ParseError;

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Token {
    Dot,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Pipe,
    Star,
    Eq,           // ==
    Ne,           // !=
    Ident(String),
    Integer(usize),
    Float(f64),
    StringLit(String),
}

pub(super) struct Tokenizer<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Tokenizer<'a> {
    pub(super) fn new(input: &'a str) -> Self {
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
            '=' => {
                self.advance_char();
                if self.peek_char() == Some('=') {
                    self.advance_char();
                    Token::Eq
                } else {
                    return Err(ParseError {
                        position: start,
                        message: "unexpected '='; did you mean '=='?".to_string(),
                    });
                }
            }
            '!' => {
                self.advance_char();
                if self.peek_char() == Some('=') {
                    self.advance_char();
                    Token::Ne
                } else {
                    return Err(ParseError {
                        position: start,
                        message: "unexpected '!'; did you mean '!='?".to_string(),
                    });
                }
            }
            '"' => {
                self.advance_char(); // consume opening quote
                let mut s = String::new();
                loop {
                    match self.peek_char() {
                        Some('"') => { self.advance_char(); break; }
                        Some('\\') => {
                            self.advance_char();
                            match self.peek_char() {
                                Some(c) => { self.advance_char(); s.push(c); }
                                None => return Err(ParseError {
                                    position: self.pos,
                                    message: "unexpected end of input in string literal".to_string(),
                                }),
                            }
                        }
                        Some(c) => { self.advance_char(); s.push(c); }
                        None => return Err(ParseError {
                            position: self.pos,
                            message: "unterminated string literal".to_string(),
                        }),
                    }
                }
                Token::StringLit(s)
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

    pub(super) fn tokenize(mut self) -> Result<Vec<(Token, usize)>, ParseError> {
        let mut tokens = Vec::new();
        while let Some(tok) = self.next_token()? {
            tokens.push(tok);
        }
        Ok(tokens)
    }
}
