#[derive(Default, Debug, Copy, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub start: i32,
    pub end: i32,
    pub size: i32,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum TokenKind {
    Undefined,
    Object,
    Array,
    Str,
    Primitive,
}

impl Default for TokenKind {
    fn default() -> Self {
        Self::Undefined
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Error {
    /// The string is not a full JSON packet, more bytes expected
    Part,
    /// Invalid character inside JSON string
    Invalid,
    /// Not enough tokens were provided
    NoMemory,
}

#[derive(Default)]
pub struct JsonParser {
    pub pos: u32,
    pub tok_next: u32,
    pub tok_super: i32,
}

impl JsonParser {
    pub fn new() -> Self {
        Self {
            tok_super: -1,
            ..Default::default()
        }
    }

    ///
    /// Run JSON parser. It parses a JSON data string into and array of tokens, each
    /// describing a single JSON object.
    ///
    /// Parse JSON string and fill tokens.
    pub fn parse(&mut self, js: &[u8], tokens: &mut [Token]) -> Result<i32, Error> {
        let mut count = self.tok_next as i32;
        while (self.pos as usize) < js.len() {
            let c = js[self.pos as usize];
            match c {
                b'{' | b'[' => {
                    count += 1;
                    let i = self.alloc_token(tokens).ok_or(Error::NoMemory)?;
                    if self.tok_super != -1 {
                        tokens[self.tok_super as usize].size += 1
                    }
                    let token = &mut tokens[i];
                    token.kind = if c == b'{' {
                        TokenKind::Object
                    } else {
                        TokenKind::Array
                    };
                    token.start = self.pos as i32;
                    self.tok_super = self.tok_next as i32 - 1;
                }
                b'}' | b']' => {
                    let kind = if c == b'}' {
                        TokenKind::Object
                    } else {
                        TokenKind::Array
                    };
                    let mut i = (self.tok_next - 1) as i32;
                    while i >= 0 {
                        let token = &mut tokens[i as usize];
                        if token.start != -1 && token.end == -1 {
                            if token.kind != kind {
                                return Err(Error::Invalid);
                            }
                            self.tok_super = -1;
                            token.end = self.pos as i32 + 1;
                            break;
                        } else {
                            i -= 1
                        }
                    }
                    // Error if unmatched closing bracket
                    if i == -1 {
                        return Err(Error::Invalid);
                    }
                    while i >= 0 {
                        let token = &mut tokens[i as usize];
                        if token.start != -1 && token.end == -1 {
                            self.tok_super = i;
                            break;
                        } else {
                            i -= 1
                        }
                    }
                }
                b'"' => {
                    let r = self.jsmn_parse_string(js, tokens)?;
                    if r < 0 {
                        return Ok(r);
                    }
                    count += 1;
                    if self.tok_super != -1 {
                        tokens[self.tok_super as usize].size += 1
                    }
                }
                b'\t' | b'\r' | b'\n' | b' ' => {}
                b':' => self.tok_super = self.tok_next as i32 - 1,
                b',' => {
                    if self.tok_super != -1
                        && tokens[self.tok_super as usize].kind != TokenKind::Array
                        && tokens[self.tok_super as usize].kind != TokenKind::Object
                    {
                        let mut i = self.tok_next as i32 - 1;
                        while i >= 0 {
                            let t = &tokens[i as usize];
                            if let TokenKind::Array | TokenKind::Object = t.kind {
                                if t.start != -1 && t.end == -1 {
                                    self.tok_super = i;
                                    break;
                                }
                            }
                            i -= 1
                        }
                    }
                }
                _ => {
                    // In non-strict mode every unquoted value is a primitive
                    let r = self.jsmn_parse_primitive(js, tokens)?;
                    if r < 0 {
                        return Ok(r);
                    }
                    count += 1;
                    if self.tok_super != -1 {
                        tokens[self.tok_super as usize].size += 1
                    }
                }
            }
            self.pos += 1;
        }
        let mut i = self.tok_next as i32 - 1;
        while i >= 0 {
            // Unmatched opened object or array
            if tokens[i as usize].start != -1 && tokens[i as usize].end == -1 {
                return Err(Error::Part);
            }
            i -= 1
        }
        Ok(count)
    }

    /// Fills next available token with JSON primitive.
    fn jsmn_parse_primitive(&mut self, js: &[u8], tokens: &mut [Token]) -> Result<i32, Error> {
        let start = self.pos as i32;
        while (self.pos as usize) < js.len() {
            match js[self.pos as usize] {
                b':' | b'\t' | b'\r' | b'\n' | b' ' | b',' | b']' | b'}' => break,
                _ => {}
            }

            if js[self.pos as usize] < 32 || js[self.pos as usize] >= 127 {
                self.pos = start as u32;
                return Err(Error::Invalid);
            }
            self.pos += 1;
        }

        match self.alloc_token(tokens) {
            Some(i) => {
                fill_token(&mut tokens[i], TokenKind::Primitive, start, self.pos as i32);
            }
            None => {
                self.pos = start as u32;
                return Err(Error::NoMemory);
            }
        }

        self.pos -= 1;
        Ok(0)
    }

    /// Fills next token with JSON string.
    fn jsmn_parse_string(&mut self, js: &[u8], tokens: &mut [Token]) -> Result<i32, Error> {
        let start: i32 = self.pos as i32;
        self.pos += 1;
        // Skip starting quote
        while (self.pos as usize) < js.len() {
            let c = js[self.pos as usize];
            // Quote: end of string
            if c == b'\"' {
                let token = self.alloc_token(tokens);
                if token.is_none() {
                    self.pos = start as u32;
                    return Err(Error::NoMemory);
                }
                match token {
                    Some(i) => fill_token(&mut tokens[i], TokenKind::Str, start + 1, self.pos as _),
                    None => {
                        self.pos = start as _;
                        return Err(Error::NoMemory);
                    }
                };
                return Ok(0);
            }
            // Backslash: Quoted symbol expected
            if c as i32 == '\\' as i32 && (self.pos as usize + 1) < js.len() {
                self.pos += 1;
                match js[self.pos as usize] {
                    b'"' | b'/' | b'\\' | b'b' | b'f' | b'r' | b'n' | b't' => {}
                    b'u' => {
                        // Allows escaped symbol \uXXXX
                        self.pos += 1;
                        let mut i = 0;
                        while i < 4 && (self.pos as usize) < js.len() {
                            // If it isn't a hex character we have an error

                            let is_hex = match js[self.pos as usize] {
                                _c @ b'0'..=b'9' => true,
                                _c @ b'A'..=b'F' => true,
                                _c @ b'a'..=b'f' => true,
                                _ => false,
                            };
                            if !is_hex {
                                self.pos = start as u32;
                                return Err(Error::Invalid);
                            }
                            self.pos += 1;
                            i += 1
                        }
                        self.pos -= 1;
                    }
                    _ => {
                        /* Unexpected symbol */
                        self.pos = start as u32;
                        return Err(Error::Invalid);
                    }
                }
            }
            self.pos += 1;
        }
        self.pos = start as u32;
        Err(Error::Part)
    }

    /// Allocates a fresh unused token from the token pool.
    fn alloc_token(&mut self, tokens: &mut [Token]) -> Option<usize> {
        if self.tok_next as usize >= tokens.len() {
            return None;
        }
        let idx = self.tok_next as usize;
        self.tok_next += 1;
        let tok = &mut tokens[idx];
        tok.end = -1;
        tok.start = tok.end;
        tok.size = 0;
        Some(idx)
    }
}

/// Fills token type and boundaries.
fn fill_token(token: &mut Token, kind: TokenKind, start: i32, end: i32) {
    token.kind = kind;
    token.start = start;
    token.end = end;
    token.size = 0;
}
