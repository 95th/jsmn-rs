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
    pub start: i32,
    pub end: i32,
    pub size: i32,
}

impl Token {
    pub fn new(kind: TokenKind, start: i32, end: i32) -> Self {
        Self::with_size(kind, start, end, 0)
    }

    pub fn with_size(kind: TokenKind, start: i32, end: i32, size: i32) -> Self {
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
    pos: u32,
    tok_next: u32,
    tok_super: i32,
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
    ///
    /// Returns number of tokens parsed.
    pub fn parse(&mut self, js: &[u8], tokens: &mut [Token]) -> Result<u32, Error> {
        let mut count = self.tok_next;
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
                    self.parse_string(js, tokens)?;
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
                    self.parse_primitive(js, tokens)?;
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
    fn parse_primitive(&mut self, js: &[u8], tokens: &mut [Token]) -> Result<(), Error> {
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
                tokens[i] = Token::new(TokenKind::Primitive, start, self.pos as i32);
            }
            None => {
                self.pos = start as u32;
                return Err(Error::NoMemory);
            }
        }

        self.pos -= 1;
        Ok(())
    }

    /// Fills next token with JSON string.
    fn parse_string(&mut self, js: &[u8], tokens: &mut [Token]) -> Result<(), Error> {
        let start: i32 = self.pos as i32;
        self.pos += 1;
        // Skip starting quote
        while (self.pos as usize) < js.len() {
            let c = js[self.pos as usize];
            // Quote: end of string
            if c == b'\"' {
                match self.alloc_token(tokens) {
                    Some(i) => tokens[i] = Token::new(TokenKind::Str, start + 1, self.pos as i32),
                    None => {
                        self.pos = start as u32;
                        return Err(Error::NoMemory);
                    }
                };
                return Ok(());
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(buf: &[u8], len: usize) -> Vec<Token> {
        let mut v = vec![Token::default(); len];
        let mut parser = JsonParser::new();
        let parsed = parser.parse(buf, &mut v).unwrap();
        assert_eq!(len, parsed as usize);
        v
    }

    #[test]
    fn parse_int() {
        let s = b"1234";
        let tokens = parse(s, 1);
        assert_eq!(vec![Token::new(TokenKind::Primitive, 0, 4)], tokens);
    }

    #[test]
    fn parse_int_negative() {
        let s = b"-1234";
        let tokens = parse(s, 1);
        assert_eq!(vec![Token::new(TokenKind::Primitive, 0, 5)], tokens);
    }

    #[test]
    fn parse_string() {
        let s = br#""abcd""#;
        let tokens = parse(s, 1);
        assert_eq!(vec![Token::new(TokenKind::Str, 1, 5)], tokens);
    }

    #[test]
    fn parse_object() {
        let s = br#"{"a": "b", "c": 100}"#;
        let tokens = parse(s, 5);
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
