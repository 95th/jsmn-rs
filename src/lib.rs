#[derive(Debug, Copy, Clone)]
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
unsafe extern "C" fn jsmn_alloc_token(
    mut parser: *mut JsmnParser,
    mut tokens: *mut JsmnToken,
    num_tokens: usize,
) -> *mut JsmnToken {
    let mut tok: *mut JsmnToken = 0 as *mut JsmnToken;
    if (*parser).tok_next as usize >= num_tokens {
        return 0 as *mut JsmnToken;
    }
    let fresh0 = (*parser).tok_next;
    (*parser).tok_next = (*parser).tok_next.wrapping_add(1);
    tok = &mut *tokens.offset(fresh0 as isize) as *mut JsmnToken;
    (*tok).end = -(1 as i32);
    (*tok).start = (*tok).end;
    (*tok).size = 0 as i32;
    return tok;
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
unsafe extern "C" fn jsmn_parse_primitive(
    parser: &mut JsmnParser,
    mut js: &[u8],
    mut tokens: *mut JsmnToken,
    num_tokens: usize,
) -> Result<i32, JsmnError> {
    let mut token: *mut JsmnToken = 0 as *mut JsmnToken;
    let mut start: i32 = 0;
    start = parser.pos as i32;
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

    /* In strict mode primitive must be followed by "," or "}" or "]" */
    if tokens.is_null() {
        parser.pos = parser.pos.wrapping_sub(1);
        return Ok(0);
    }
    token = jsmn_alloc_token(parser, tokens, num_tokens);
    if token.is_null() {
        parser.pos = start as u32;
        return Err(JsmnError::NoMemory);
    }
    jsmn_fill_token(&mut *token, JsmnType::Primitive, start, parser.pos as i32);
    parser.pos = parser.pos.wrapping_sub(1);
    Ok(0)
}
/* *
 * Fills next token with JSON string.
 */
unsafe extern "C" fn jsmn_parse_string(
    mut parser: *mut JsmnParser,
    mut js: *const u8,
    len: usize,
    mut tokens: *mut JsmnToken,
    num_tokens: usize,
) -> i32 {
    let mut token: *mut JsmnToken = 0 as *mut JsmnToken;
    let mut start: i32 = (*parser).pos as i32;
    (*parser).pos = (*parser).pos.wrapping_add(1);
    /* Skip starting quote */
    while ((*parser).pos as usize) < len
        && *js.offset((*parser).pos as isize) as i32 != '\u{0}' as i32
    {
        let mut c: u8 = *js.offset((*parser).pos as isize);
        /* Quote: end of string */
        if c as i32 == '\"' as i32 {
            if tokens.is_null() {
                return 0 as i32;
            }
            token = jsmn_alloc_token(parser, tokens, num_tokens);
            if token.is_null() {
                (*parser).pos = start as u32;
                return JsmnError::NoMemory as _;
            }
            jsmn_fill_token(
                &mut *token,
                JsmnType::Str,
                start + 1 as i32,
                (*parser).pos as i32,
            );
            return 0 as i32;
        }
        /* Backslash: Quoted symbol expected */
        if c as i32 == '\\' as i32 && ((*parser).pos.wrapping_add(1 as i32 as u32) as usize) < len {
            let mut i: i32 = 0;
            (*parser).pos = (*parser).pos.wrapping_add(1);
            match *js.offset((*parser).pos as isize) as i32 {
                34 | 47 | 92 | 98 | 102 | 114 | 110 | 116 => {}
                117 => {
                    /* Allows escaped symbol \uXXXX */
                    (*parser).pos = (*parser).pos.wrapping_add(1);
                    i = 0 as i32;
                    while i < 4 as i32
                        && ((*parser).pos as usize) < len
                        && *js.offset((*parser).pos as isize) as i32 != '\u{0}' as i32
                    {
                        /* If it isn't a hex character we have an error */
                        if !(*js.offset((*parser).pos as isize) as i32 >= 48 as i32
                            && *js.offset((*parser).pos as isize) as i32 <= 57 as i32
                            || *js.offset((*parser).pos as isize) as i32 >= 65 as i32
                                && *js.offset((*parser).pos as isize) as i32 <= 70 as i32
                            || *js.offset((*parser).pos as isize) as i32 >= 97 as i32
                                && *js.offset((*parser).pos as isize) as i32 <= 102 as i32)
                        {
                            /* a-f */
                            (*parser).pos = start as u32;
                            return JsmnError::Invalid as _;
                        }
                        (*parser).pos = (*parser).pos.wrapping_add(1);
                        i += 1
                    }
                    (*parser).pos = (*parser).pos.wrapping_sub(1)
                }
                _ => {
                    /* Unexpected symbol */
                    (*parser).pos = start as u32;
                    return JsmnError::Invalid as _;
                }
            }
        }
        (*parser).pos = (*parser).pos.wrapping_add(1)
    }
    (*parser).pos = start as u32;
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
#[no_mangle]
pub unsafe extern "C" fn jsmn_parse(
    mut parser: &mut JsmnParser,
    js: &[u8],
    tokens: *mut JsmnToken,
    num_tokens: u32,
) -> Result<i32, JsmnError> {
    let mut token;
    let mut count: i32 = parser.tok_next as i32;
    while (parser.pos as usize) < js.len() {
        let c = js[parser.pos as usize];
        match c as i32 {
            123 | 91 => {
                count += 1;
                if !tokens.is_null() {
                    token = jsmn_alloc_token(parser, tokens, num_tokens as usize);
                    if token.is_null() {
                        return Err(JsmnError::NoMemory);
                    }
                    if parser.tok_super != -(1 as i32) {
                        let mut t: *mut JsmnToken =
                            &mut *tokens.offset(parser.tok_super as isize) as *mut JsmnToken;
                        (*t).size += 1
                    }
                    (*token).type_0 = if c as i32 == '{' as i32 {
                        JsmnType::Object
                    } else {
                        JsmnType::Array
                    };
                    (*token).start = parser.pos as i32;
                    parser.tok_super = parser.tok_next.wrapping_sub(1 as i32 as u32) as i32
                }
            }
            125 | 93 => {
                if !tokens.is_null() {
                    let type_0 = if c as i32 == '}' as i32 {
                        JsmnType::Object
                    } else {
                        JsmnType::Array
                    };
                    let mut i = parser.tok_next.wrapping_sub(1 as i32 as u32) as i32;
                    while i >= 0 as i32 {
                        token = &mut *tokens.offset(i as isize) as *mut JsmnToken;
                        if (*token).start != -(1 as i32) && (*token).end == -(1 as i32) {
                            if (*token).type_0 as u32 != type_0 as u32 {
                                return Err(JsmnError::Invalid);
                            }
                            parser.tok_super = -(1 as i32);
                            (*token).end = parser.pos.wrapping_add(1 as i32 as u32) as i32;
                            break;
                        } else {
                            i -= 1
                        }
                    }
                    /* Error if unmatched closing bracket */
                    if i == -(1 as i32) {
                        return Err(JsmnError::Invalid);
                    }
                    while i >= 0 as i32 {
                        token = &mut *tokens.offset(i as isize) as *mut JsmnToken;
                        if (*token).start != -(1 as i32) && (*token).end == -(1 as i32) {
                            parser.tok_super = i;
                            break;
                        } else {
                            i -= 1
                        }
                    }
                }
            }
            34 => {
                let r =
                    jsmn_parse_string(parser, js.as_ptr(), js.len(), tokens, num_tokens as usize);
                if r < 0 {
                    return Ok(r);
                }
                count += 1;
                if parser.tok_super != -(1 as i32) && !tokens.is_null() {
                    let ref mut fresh1 = (*tokens.offset(parser.tok_super as isize)).size;
                    *fresh1 += 1
                }
            }
            9 | 13 | 10 | 32 => {}
            58 => parser.tok_super = parser.tok_next.wrapping_sub(1 as i32 as u32) as i32,
            44 => {
                if !tokens.is_null()
                    && parser.tok_super != -(1 as i32)
                    && (*tokens.offset(parser.tok_super as isize)).type_0 as u32
                        != JsmnType::Array as i32 as u32
                    && (*tokens.offset(parser.tok_super as isize)).type_0 as u32
                        != JsmnType::Object as i32 as u32
                {
                    let mut i = parser.tok_next.wrapping_sub(1 as i32 as u32) as i32;
                    while i >= 0 as i32 {
                        if (*tokens.offset(i as isize)).type_0 as u32
                            == JsmnType::Array as i32 as u32
                            || (*tokens.offset(i as isize)).type_0 as u32
                                == JsmnType::Object as i32 as u32
                        {
                            if (*tokens.offset(i as isize)).start != -(1 as i32)
                                && (*tokens.offset(i as isize)).end == -(1 as i32)
                            {
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
                let r = jsmn_parse_primitive(&mut *parser, js, tokens, num_tokens as usize)?;
                if r < 0 {
                    return Ok(r);
                }
                count += 1;
                if parser.tok_super != -(1 as i32) && !tokens.is_null() {
                    let ref mut fresh2 = (*tokens.offset(parser.tok_super as isize)).size;
                    *fresh2 += 1
                }
            }
        }
        parser.pos = parser.pos.wrapping_add(1)
    }
    if !tokens.is_null() {
        let mut i = parser.tok_next.wrapping_sub(1 as i32 as u32) as i32;
        while i >= 0 as i32 {
            /* Unmatched opened object or array */
            if (*tokens.offset(i as isize)).start != -(1 as i32)
                && (*tokens.offset(i as isize)).end == -(1 as i32)
            {
                return Err(JsmnError::Part);
            }
            i -= 1
        }
    }
    Ok(count)
}
/* JSMN_H */
/* JSMN_HEADER */
