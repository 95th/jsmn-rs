/*
 * MIT License
 *
 * Copyright (c) 2010 Serge Zaitsev
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

//! jsmn (pronounced like 'jasmine') is Rust port of a minimalistic JSON parser.
//! It can be easily integrated into resource-limited or embedded projects.
//!
//! # Philosophy
//!
//! Most JSON parsers offer you a bunch of functions to load JSON data, parse it and
//! extract any value by its name. jsmn proves that checking the correctness of every
//! JSON packet or allocating temporary objects to store parsed JSON fields often is
//! an overkill.
//!
//! JSON format itself is extremely simple, so why should we complicate it?
//!
//! jsmn is designed to be robust (it should work fine even with erroneous data), fast
//! (it should parse data on the fly), portable. And of course, simplicity is a key feature
//! - simple code style, simple algorithm, simple integration into other projects.

#[derive(Default, Debug, Copy, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub start: isize,
    pub end: isize,
    pub size: usize,
}

impl Token {
    pub fn new(kind: TokenKind, start: isize, end: isize) -> Self {
        Self::with_size(kind, start, end, 0)
    }

    pub fn with_size(kind: TokenKind, start: isize, end: isize, size: usize) -> Self {
        Self {
            kind,
            start,
            end,
            size,
        }
    }
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

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Error {
    /// The string is not a full JSON packet, more bytes expected
    Part,
    /// Invalid character inside JSON string
    Invalid,
    /// Not enough tokens were provided
    NoMemory,
}

pub struct JsonParser {
    pos: usize,
    tok_next: usize,
    tok_super: Option<usize>,
}

impl Default for JsonParser {
    fn default() -> Self {
        Self {
            pos: 0,
            tok_next: 0,
            tok_super: None,
        }
    }
}

impl JsonParser {
    pub fn new() -> Self {
        Self::default()
    }

    ///
    /// Run JSON parser. It parses a JSON data string into and array of tokens, each
    /// describing a single JSON object.
    ///
    /// Parse JSON string and fill tokens.
    ///
    /// Returns number of tokens parsed.
    pub fn parse(&mut self, js: &[u8], tokens: &mut [Token]) -> Result<usize, Error> {
        let mut count = self.tok_next;
        while self.pos < js.len() {
            let c = js[self.pos];
            match c {
                b'{' | b'[' => {
                    count += 1;
                    let i = self.alloc_token(tokens).ok_or(Error::NoMemory)?;
                    if let Some(i) = self.tok_super {
                        let t = &mut tokens[i];
                        // An object or array can't become a key
                        if let TokenKind::Object | TokenKind::Array = t.kind {
                            return Err(Error::Invalid);
                        }
                        t.size += 1
                    }
                    let token = &mut tokens[i];
                    token.kind = if c == b'{' {
                        TokenKind::Object
                    } else {
                        TokenKind::Array
                    };
                    token.start = self.pos as _;
                    self.tok_super = Some(self.tok_next - 1);
                }
                b'}' | b']' => {
                    let kind = if c == b'}' {
                        TokenKind::Object
                    } else {
                        TokenKind::Array
                    };
                    let mut i = (self.tok_next - 1) as isize;
                    while i >= 0 {
                        let token = &mut tokens[i as usize];
                        if token.start != -1 && token.end == -1 {
                            if token.kind != kind {
                                return Err(Error::Invalid);
                            }
                            self.tok_super = None;
                            token.end = self.pos as isize + 1;
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
                            self.tok_super = Some(i as usize);
                            break;
                        } else {
                            i -= 1
                        }
                    }
                }
                b'"' => {
                    self.parse_string(js, tokens)?;
                    count += 1;
                    if let Some(i) = self.tok_super {
                        tokens[i].size += 1
                    }
                }
                b'\t' | b'\r' | b'\n' | b' ' => {}
                b':' => self.tok_super = Some(self.tok_next - 1),
                b',' => {
                    if let Some(i) = self.tok_super {
                        match tokens[i].kind {
                            TokenKind::Array | TokenKind::Object => {}
                            _ => {
                                let mut i = self.tok_next as isize - 1;
                                while i >= 0 {
                                    let t = &tokens[i as usize];
                                    if let TokenKind::Array | TokenKind::Object = t.kind {
                                        if t.start != -1 && t.end == -1 {
                                            self.tok_super = Some(i as usize);
                                            break;
                                        }
                                    }
                                    i -= 1
                                }
                            }
                        }
                    }
                }
                b'0'..=b'9' | b'-' | b't' | b'f' | b'n' => {
                    // Primitives are: numbers and booleans and
                    // they must not be keys of the object
                    if let Some(i) = self.tok_super {
                        let t = &mut tokens[i];
                        match t.kind {
                            TokenKind::Object => return Err(Error::Invalid),
                            TokenKind::Str if t.size != 0 => return Err(Error::Invalid),
                            _ => {}
                        }
                    }
                    self.parse_primitive(js, tokens)?;
                    count += 1;
                    if let Some(i) = self.tok_super {
                        tokens[i].size += 1
                    }
                }
                _ => {
                    // Unexpected char
                    return Err(Error::Invalid);
                }
            }
            self.pos += 1;
        }
        let mut i = self.tok_next as isize - 1;
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
    fn parse_primitive(&mut self, js: &[u8], tokens: &mut [Token]) -> Result<(), Error> {
        let start = self.pos as isize;
        while self.pos < js.len() {
            match js[self.pos] {
                b':' | b'\t' | b'\r' | b'\n' | b' ' | b',' | b']' | b'}' => break,
                _ => {}
            }

            if js[self.pos] < 32 || js[self.pos] >= 127 {
                self.pos = start as _;
                return Err(Error::Invalid);
            }
            self.pos += 1;
        }

        match self.alloc_token(tokens) {
            Some(i) => {
                tokens[i] = Token::new(TokenKind::Primitive, start, self.pos as _);
            }
            None => {
                self.pos = start as _;
                return Err(Error::NoMemory);
            }
        }

        self.pos -= 1;
        Ok(())
    }

    /// Fills next token with JSON string.
    fn parse_string(&mut self, js: &[u8], tokens: &mut [Token]) -> Result<(), Error> {
        let start = self.pos as isize;
        self.pos += 1;
        // Skip starting quote
        while self.pos < js.len() {
            let c = js[self.pos];
            // Quote: end of string
            if c == b'\"' {
                match self.alloc_token(tokens) {
                    Some(i) => tokens[i] = Token::new(TokenKind::Str, start + 1, self.pos as _),
                    None => {
                        self.pos = start as _;
                        return Err(Error::NoMemory);
                    }
                };
                return Ok(());
            }
            // Backslash: Quoted symbol expected
            if c == b'\\' && (self.pos + 1) < js.len() {
                self.pos += 1;
                match js[self.pos] {
                    b'"' | b'/' | b'\\' | b'b' | b'f' | b'r' | b'n' | b't' => {}
                    b'u' => {
                        // Allows escaped symbol \uXXXX
                        self.pos += 1;
                        let mut i = 0;
                        while i < 4 && self.pos < js.len() {
                            // If it isn't a hex character we have an error

                            let is_hex = match js[self.pos] {
                                _c @ b'0'..=b'9' => true,
                                _c @ b'A'..=b'F' => true,
                                _c @ b'a'..=b'f' => true,
                                _ => false,
                            };
                            if !is_hex {
                                self.pos = start as _;
                                return Err(Error::Invalid);
                            }
                            self.pos += 1;
                            i += 1
                        }
                        self.pos -= 1;
                    }
                    _ => {
                        /* Unexpected symbol */
                        self.pos = start as _;
                        return Err(Error::Invalid);
                    }
                }
            }
            self.pos += 1;
        }
        self.pos = start as _;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(buf: &[u8], len: usize) -> Result<Vec<Token>, Error> {
        let mut v = vec![Token::default(); len];
        let mut parser = JsonParser::new();
        let parsed = parser.parse(buf, &mut v)?;
        assert_eq!(len, parsed as usize);
        Ok(v)
    }

    #[test]
    fn parse_int() {
        let s = b"1234";
        let tokens = parse(s, 1).unwrap();
        assert_eq!(vec![Token::new(TokenKind::Primitive, 0, 4)], tokens);
    }

    #[test]
    fn parse_int_negative() {
        let s = b"-1234";
        let tokens = parse(s, 1).unwrap();
        assert_eq!(vec![Token::new(TokenKind::Primitive, 0, 5)], tokens);
    }

    #[test]
    fn parse_int_invalid() {
        let s = b"abc1234";
        let err = parse(s, 1).unwrap_err();
        assert_eq!(Error::Invalid, err);
    }

    #[test]
    fn parse_string() {
        let s = br#""abcd""#;
        let tokens = parse(s, 1).unwrap();
        assert_eq!(vec![Token::new(TokenKind::Str, 1, 5)], tokens);
    }

    #[test]
    fn parse_object() {
        let s = br#"{"a": "b", "c": 100}"#;
        let tokens = parse(s, 5).unwrap();
        assert_eq!(
            vec![
                Token::with_size(TokenKind::Object, 0, 20, 2),
                Token::with_size(TokenKind::Str, 2, 3, 1),
                Token::with_size(TokenKind::Str, 7, 8, 0),
                Token::with_size(TokenKind::Str, 12, 13, 1),
                Token::with_size(TokenKind::Primitive, 16, 19, 0)
            ],
            tokens
        );
    }
}
