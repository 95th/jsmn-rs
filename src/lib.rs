#[derive(Debug, Copy, Clone, PartialEq)]
pub enum JsmnType {
    Undefined,
    Object,
    Array,
    Str,
    Primitive,
}

impl Default for JsmnType {
    fn default() -> Self {
        Self::Undefined
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(i32)]
pub enum JsmnError {
    /// The string is not a full JSON packet, more bytes expected
    Part,
    /// Invalid character inside JSON string
    Invalid,
    /// Not enough tokens were provided
    NoMemory,
}

#[derive(Default, Debug, Copy, Clone)]
#[repr(C)]
pub struct JsmnToken {
    pub type_0: JsmnType,
    pub start: i32,
    pub end: i32,
    pub size: i32,
}

pub struct JsmnParser {
    pub pos: u32,
    pub tok_next: u32,
    pub tok_super: i32,
}

impl JsmnParser {
    pub fn new() -> Self {
        Self {
            pos: 0,
            tok_next: 0,
            tok_super: -1,
        }
    }
}
/* *
 * Allocates a fresh unused token from the token pool.
 */
fn jsmn_alloc_token(mut parser: &mut JsmnParser, tokens: &mut [JsmnToken]) -> Option<usize> {
    if parser.tok_next as usize >= tokens.len() {
        return None;
    }
    let idx = parser.tok_next as usize;
    parser.tok_next += 1;
    let tok = &mut tokens[idx];
    tok.end = -1;
    tok.start = tok.end;
    tok.size = 0;
    return Some(idx);
}
/* *
 * Fills token type and boundaries.
 */
fn jsmn_fill_token(token: &mut JsmnToken, type_0: JsmnType, start: i32, end: i32) {
    token.type_0 = type_0;
    token.start = start;
    token.end = end;
    token.size = 0;
}
/* *
 * Fills next available token with JSON primitive.
 */
fn jsmn_parse_primitive(
    parser: &mut JsmnParser,
    js: &[u8],
    tokens: &mut [JsmnToken],
) -> Result<i32, JsmnError> {
    let start = parser.pos as i32;
    while (parser.pos as usize) < js.len() {
        match js[parser.pos as usize] {
            b':' | b'\t' | b'\r' | b'\n' | b' ' | b',' | b']' | b'}' => break,
            _ => {}
        }

        if (js[parser.pos as usize]) < 32 || js[parser.pos as usize] >= 127 {
            parser.pos = start as u32;
            return Err(JsmnError::Invalid);
        }
        parser.pos += 1;
    }

    let token = jsmn_alloc_token(parser, tokens);
    if token.is_none() {
        parser.pos = start as u32;
        return Err(JsmnError::NoMemory);
    }
    jsmn_fill_token(
        &mut tokens[token.unwrap()],
        JsmnType::Primitive,
        start,
        parser.pos as i32,
    );
    parser.pos -= 1;
    Ok(0)
}
/* *
 * Fills next token with JSON string.
 */
fn jsmn_parse_string(mut parser: &mut JsmnParser, js: &[u8], tokens: &mut [JsmnToken]) -> i32 {
    let start: i32 = parser.pos as i32;
    parser.pos += 1;
    /* Skip starting quote */
    while (parser.pos as usize) < js.len() {
        let c = js[parser.pos as usize];
        /* Quote: end of string */
        if c == b'\"' {
            let token = jsmn_alloc_token(parser, tokens);
            if token.is_none() {
                parser.pos = start as u32;
                return JsmnError::NoMemory as _;
            }
            match token {
                Some(i) => {
                    jsmn_fill_token(&mut tokens[i], JsmnType::Str, start + 1, parser.pos as _)
                }
                None => {
                    parser.pos = start as _;
                    return JsmnError::NoMemory as _;
                }
            };
            return 0 as i32;
        }
        /* Backslash: Quoted symbol expected */
        if c as i32 == '\\' as i32 && (parser.pos as usize + 1) < js.len() {
            parser.pos += 1;
            match js[parser.pos as usize] {
                b'"' | b'/' | b'\\' | b'b' | b'f' | b'r' | b'n' | b't' => {}
                b'u' => {
                    /* Allows escaped symbol \uXXXX */
                    parser.pos += 1;
                    let mut i = 0;
                    while i < 4 && (parser.pos as usize) < js.len() {
                        /* If it isn't a hex character we have an error */

                        let is_hex = match js[parser.pos as usize] {
                            _c @ b'0'..=b'9' => true,
                            _c @ b'A'..=b'F' => true,
                            _c @ b'a'..=b'f' => true,
                            _ => false,
                        };
                        if !is_hex {
                            parser.pos = start as u32;
                            return JsmnError::Invalid as _;
                        }
                        parser.pos += 1;
                        i += 1
                    }
                    parser.pos -= 1;
                }
                _ => {
                    /* Unexpected symbol */
                    parser.pos = start as u32;
                    return JsmnError::Invalid as _;
                }
            }
        }
        parser.pos += 1;
    }
    parser.pos = start as u32;
    return JsmnError::Part as _;
}
/* *
 * Run JSON parser. It parses a JSON data string into and array of tokens, each
 * describing
 * a single JSON object.
 */
/* *
 * Parse JSON string and fill tokens.
 */
pub fn jsmn_parse(
    mut parser: &mut JsmnParser,
    js: &[u8],
    tokens: &mut [JsmnToken],
) -> Result<i32, JsmnError> {
    let mut count = parser.tok_next as i32;
    while (parser.pos as usize) < js.len() {
        let c = js[parser.pos as usize];
        match c {
            b'{' | b'[' => {
                count += 1;
                let token = jsmn_alloc_token(parser, tokens);
                if token.is_none() {
                    return Err(JsmnError::NoMemory);
                }
                if parser.tok_super != -1 {
                    let mut t = &mut tokens[parser.tok_super as usize];
                    t.size += 1
                }
                let token = &mut tokens[token.unwrap()];
                token.type_0 = if c == b'{' {
                    JsmnType::Object
                } else {
                    JsmnType::Array
                };
                token.start = parser.pos as i32;
                parser.tok_super = parser.tok_next as i32 - 1;
            }
            b'}' | b']' => {
                let type_0 = if c == b'}' {
                    JsmnType::Object
                } else {
                    JsmnType::Array
                };
                let mut i = (parser.tok_next - 1) as i32;
                while i >= 0 as i32 {
                    let token = &mut tokens[i as usize];
                    if token.start != -1 && token.end == -1 {
                        if token.type_0 as u32 != type_0 as u32 {
                            return Err(JsmnError::Invalid);
                        }
                        parser.tok_super = -1;
                        token.end = parser.pos as i32 + 1;
                        break;
                    } else {
                        i -= 1
                    }
                }
                /* Error if unmatched closing bracket */
                if i == -1 {
                    return Err(JsmnError::Invalid);
                }
                while i >= 0 as i32 {
                    let token = &mut tokens[i as usize];
                    if token.start != -1 && token.end == -1 {
                        parser.tok_super = i;
                        break;
                    } else {
                        i -= 1
                    }
                }
            }
            b'"' => {
                let r = jsmn_parse_string(parser, js, tokens);
                if r < 0 {
                    return Ok(r);
                }
                count += 1;
                if parser.tok_super != -1 {
                    tokens[parser.tok_super as usize].size += 1
                }
            }
            b'\t' | b'\r' | b'\n' | b' ' => {}
            b':' => parser.tok_super = parser.tok_next as i32 - 1,
            b',' => {
                if parser.tok_super != -1
                    && tokens[parser.tok_super as usize].type_0 != JsmnType::Array
                    && tokens[parser.tok_super as usize].type_0 != JsmnType::Object
                {
                    let mut i = parser.tok_next as i32 - 1;
                    while i >= 0 {
                        let t = &tokens[i as usize];
                        if let JsmnType::Array | JsmnType::Object = t.type_0 {
                            if t.start != -1 && t.end == -1 {
                                parser.tok_super = i;
                                break;
                            }
                        }
                        i -= 1
                    }
                }
            }
            _ => {
                /* In non-strict mode every unquoted value is a primitive */
                let r = jsmn_parse_primitive(&mut *parser, js, tokens)?;
                if r < 0 {
                    return Ok(r);
                }
                count += 1;
                if parser.tok_super != -1 {
                    tokens[parser.tok_super as usize].size += 1
                }
            }
        }
        parser.pos += 1;
    }
    let mut i = parser.tok_next as i32 - 1;
    while i >= 0 as i32 {
        /* Unmatched opened object or array */
        if tokens[i as usize].start != -1 && tokens[i as usize].end == -1 {
            return Err(JsmnError::Part);
        }
        i -= 1
    }
    Ok(count)
}
/* JSMN_H */
/* JSMN_HEADER */
