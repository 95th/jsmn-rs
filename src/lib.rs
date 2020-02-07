#![allow(
    dead_code,
    mutable_transmutes,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut
)]
pub type size_t = libc::c_ulong;
pub type jsmntype_t = libc::c_uint;
pub const JSMN_PRIMITIVE: jsmntype_t = 4;
pub const JSMN_STRING: jsmntype_t = 3;
pub const JSMN_ARRAY: jsmntype_t = 2;
pub const JSMN_OBJECT: jsmntype_t = 1;
pub const JSMN_UNDEFINED: jsmntype_t = 0;
pub type jsmnerr = libc::c_int;
/* The string is not a full JSON packet, more bytes expected */
pub const JSMN_ERROR_PART: jsmnerr = -3;
/* Invalid character inside JSON string */
pub const JSMN_ERROR_INVAL: jsmnerr = -2;
/* Not enough tokens were provided */
pub const JSMN_ERROR_NOMEM: jsmnerr = -1;
#[derive(Default, Debug, Copy, Clone)]
#[repr(C)]
pub struct JsmnToken {
    pub type_0: jsmntype_t,
    pub start: libc::c_int,
    pub end: libc::c_int,
    pub size: libc::c_int,
}

pub struct JsmnParser {
    pub pos: u32,
    pub toknext: u32,
    pub toksuper: i32,
}

impl JsmnParser {
    pub fn new() -> Self {
        Self {
            pos: 0,
            toknext: 0,
            toksuper: -1,
        }
    }
}
/* *
 * Allocates a fresh unused token from the token pool.
 */
unsafe extern "C" fn jsmn_alloc_token(
    mut parser: *mut JsmnParser,
    mut tokens: *mut JsmnToken,
    num_tokens: size_t,
) -> *mut JsmnToken {
    let mut tok: *mut JsmnToken = 0 as *mut JsmnToken;
    if (*parser).toknext as libc::c_ulong >= num_tokens {
        return 0 as *mut JsmnToken;
    }
    let fresh0 = (*parser).toknext;
    (*parser).toknext = (*parser).toknext.wrapping_add(1);
    tok = &mut *tokens.offset(fresh0 as isize) as *mut JsmnToken;
    (*tok).end = -(1 as libc::c_int);
    (*tok).start = (*tok).end;
    (*tok).size = 0 as libc::c_int;
    return tok;
}
/* *
 * Fills token type and boundaries.
 */
fn jsmn_fill_token(
    token: &mut JsmnToken,
    type_0: jsmntype_t,
    start: libc::c_int,
    end: libc::c_int,
) {
    token.type_0 = type_0;
    token.start = start;
    token.end = end;
    token.size = 0 as libc::c_int;
}
/* *
 * Fills next available token with JSON primitive.
 */
unsafe extern "C" fn jsmn_parse_primitive(
    mut parser: *mut JsmnParser,
    mut js: *const libc::c_char,
    len: size_t,
    mut tokens: *mut JsmnToken,
    num_tokens: size_t,
) -> libc::c_int {
    let mut token: *mut JsmnToken = 0 as *mut JsmnToken;
    let mut start: libc::c_int = 0;
    start = (*parser).pos as libc::c_int;
    while ((*parser).pos as libc::c_ulong) < len
        && *js.offset((*parser).pos as isize) as libc::c_int != '\u{0}' as i32
    {
        match *js.offset((*parser).pos as isize) as libc::c_int {
            58 | 9 | 13 | 10 | 32 | 44 | 93 | 125 => {
                break;
            }
            _ => {}
        }
        /* to quiet a warning from gcc*/
        if (*js.offset((*parser).pos as isize) as libc::c_int) < 32 as libc::c_int
            || *js.offset((*parser).pos as isize) as libc::c_int >= 127 as libc::c_int
        {
            (*parser).pos = start as libc::c_uint;
            return JSMN_ERROR_INVAL as libc::c_int;
        }
        (*parser).pos = (*parser).pos.wrapping_add(1)
    }
    /* In strict mode primitive must be followed by "," or "}" or "]" */
    if tokens.is_null() {
        (*parser).pos = (*parser).pos.wrapping_sub(1);
        return 0 as libc::c_int;
    }
    token = jsmn_alloc_token(parser, tokens, num_tokens);
    if token.is_null() {
        (*parser).pos = start as libc::c_uint;
        return JSMN_ERROR_NOMEM as libc::c_int;
    }
    jsmn_fill_token(
        &mut *token,
        JSMN_PRIMITIVE,
        start,
        (*parser).pos as libc::c_int,
    );
    (*parser).pos = (*parser).pos.wrapping_sub(1);
    return 0 as libc::c_int;
}
/* *
 * Fills next token with JSON string.
 */
unsafe extern "C" fn jsmn_parse_string(
    mut parser: *mut JsmnParser,
    mut js: *const libc::c_char,
    len: size_t,
    mut tokens: *mut JsmnToken,
    num_tokens: size_t,
) -> libc::c_int {
    let mut token: *mut JsmnToken = 0 as *mut JsmnToken;
    let mut start: libc::c_int = (*parser).pos as libc::c_int;
    (*parser).pos = (*parser).pos.wrapping_add(1);
    /* Skip starting quote */
    while ((*parser).pos as libc::c_ulong) < len
        && *js.offset((*parser).pos as isize) as libc::c_int != '\u{0}' as i32
    {
        let mut c: libc::c_char = *js.offset((*parser).pos as isize);
        /* Quote: end of string */
        if c as libc::c_int == '\"' as i32 {
            if tokens.is_null() {
                return 0 as libc::c_int;
            }
            token = jsmn_alloc_token(parser, tokens, num_tokens);
            if token.is_null() {
                (*parser).pos = start as libc::c_uint;
                return JSMN_ERROR_NOMEM as libc::c_int;
            }
            jsmn_fill_token(
                &mut *token,
                JSMN_STRING,
                start + 1 as libc::c_int,
                (*parser).pos as libc::c_int,
            );
            return 0 as libc::c_int;
        }
        /* Backslash: Quoted symbol expected */
        if c as libc::c_int == '\\' as i32
            && ((*parser).pos.wrapping_add(1 as libc::c_int as libc::c_uint) as libc::c_ulong) < len
        {
            let mut i: libc::c_int = 0;
            (*parser).pos = (*parser).pos.wrapping_add(1);
            match *js.offset((*parser).pos as isize) as libc::c_int {
                34 | 47 | 92 | 98 | 102 | 114 | 110 | 116 => {}
                117 => {
                    /* Allows escaped symbol \uXXXX */
                    (*parser).pos = (*parser).pos.wrapping_add(1);
                    i = 0 as libc::c_int;
                    while i < 4 as libc::c_int
                        && ((*parser).pos as libc::c_ulong) < len
                        && *js.offset((*parser).pos as isize) as libc::c_int != '\u{0}' as i32
                    {
                        /* If it isn't a hex character we have an error */
                        if !(*js.offset((*parser).pos as isize) as libc::c_int >= 48 as libc::c_int
                            && *js.offset((*parser).pos as isize) as libc::c_int
                                <= 57 as libc::c_int
                            || *js.offset((*parser).pos as isize) as libc::c_int
                                >= 65 as libc::c_int
                                && *js.offset((*parser).pos as isize) as libc::c_int
                                    <= 70 as libc::c_int
                            || *js.offset((*parser).pos as isize) as libc::c_int
                                >= 97 as libc::c_int
                                && *js.offset((*parser).pos as isize) as libc::c_int
                                    <= 102 as libc::c_int)
                        {
                            /* a-f */
                            (*parser).pos = start as libc::c_uint;
                            return JSMN_ERROR_INVAL as libc::c_int;
                        }
                        (*parser).pos = (*parser).pos.wrapping_add(1);
                        i += 1
                    }
                    (*parser).pos = (*parser).pos.wrapping_sub(1)
                }
                _ => {
                    /* Unexpected symbol */
                    (*parser).pos = start as libc::c_uint;
                    return JSMN_ERROR_INVAL as libc::c_int;
                }
            }
        }
        (*parser).pos = (*parser).pos.wrapping_add(1)
    }
    (*parser).pos = start as libc::c_uint;
    return JSMN_ERROR_PART as libc::c_int;
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
    mut parser: *mut JsmnParser,
    mut js: *const libc::c_char,
    len: size_t,
    mut tokens: *mut JsmnToken,
    num_tokens: libc::c_uint,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    let mut i: libc::c_int = 0;
    let mut token: *mut JsmnToken = 0 as *mut JsmnToken;
    let mut count: libc::c_int = (*parser).toknext as libc::c_int;
    while ((*parser).pos as libc::c_ulong) < len
        && *js.offset((*parser).pos as isize) as libc::c_int != '\u{0}' as i32
    {
        let mut c: libc::c_char = 0;
        let mut type_0: jsmntype_t = JSMN_UNDEFINED;
        c = *js.offset((*parser).pos as isize);
        match c as libc::c_int {
            123 | 91 => {
                count += 1;
                if !tokens.is_null() {
                    token = jsmn_alloc_token(parser, tokens, num_tokens as size_t);
                    if token.is_null() {
                        return JSMN_ERROR_NOMEM as libc::c_int;
                    }
                    if (*parser).toksuper != -(1 as libc::c_int) {
                        let mut t: *mut JsmnToken =
                            &mut *tokens.offset((*parser).toksuper as isize) as *mut JsmnToken;
                        (*t).size += 1
                    }
                    (*token).type_0 = if c as libc::c_int == '{' as i32 {
                        JSMN_OBJECT as libc::c_int
                    } else {
                        JSMN_ARRAY as libc::c_int
                    } as jsmntype_t;
                    (*token).start = (*parser).pos as libc::c_int;
                    (*parser).toksuper = (*parser)
                        .toknext
                        .wrapping_sub(1 as libc::c_int as libc::c_uint)
                        as libc::c_int
                }
            }
            125 | 93 => {
                if !tokens.is_null() {
                    type_0 = if c as libc::c_int == '}' as i32 {
                        JSMN_OBJECT as libc::c_int
                    } else {
                        JSMN_ARRAY as libc::c_int
                    } as jsmntype_t;
                    i = (*parser)
                        .toknext
                        .wrapping_sub(1 as libc::c_int as libc::c_uint)
                        as libc::c_int;
                    while i >= 0 as libc::c_int {
                        token = &mut *tokens.offset(i as isize) as *mut JsmnToken;
                        if (*token).start != -(1 as libc::c_int)
                            && (*token).end == -(1 as libc::c_int)
                        {
                            if (*token).type_0 as libc::c_uint != type_0 as libc::c_uint {
                                return JSMN_ERROR_INVAL as libc::c_int;
                            }
                            (*parser).toksuper = -(1 as libc::c_int);
                            (*token).end =
                                (*parser).pos.wrapping_add(1 as libc::c_int as libc::c_uint)
                                    as libc::c_int;
                            break;
                        } else {
                            i -= 1
                        }
                    }
                    /* Error if unmatched closing bracket */
                    if i == -(1 as libc::c_int) {
                        return JSMN_ERROR_INVAL as libc::c_int;
                    }
                    while i >= 0 as libc::c_int {
                        token = &mut *tokens.offset(i as isize) as *mut JsmnToken;
                        if (*token).start != -(1 as libc::c_int)
                            && (*token).end == -(1 as libc::c_int)
                        {
                            (*parser).toksuper = i;
                            break;
                        } else {
                            i -= 1
                        }
                    }
                }
            }
            34 => {
                r = jsmn_parse_string(parser, js, len, tokens, num_tokens as size_t);
                if r < 0 as libc::c_int {
                    return r;
                }
                count += 1;
                if (*parser).toksuper != -(1 as libc::c_int) && !tokens.is_null() {
                    let ref mut fresh1 = (*tokens.offset((*parser).toksuper as isize)).size;
                    *fresh1 += 1
                }
            }
            9 | 13 | 10 | 32 => {}
            58 => {
                (*parser).toksuper = (*parser)
                    .toknext
                    .wrapping_sub(1 as libc::c_int as libc::c_uint)
                    as libc::c_int
            }
            44 => {
                if !tokens.is_null()
                    && (*parser).toksuper != -(1 as libc::c_int)
                    && (*tokens.offset((*parser).toksuper as isize)).type_0 as libc::c_uint
                        != JSMN_ARRAY as libc::c_int as libc::c_uint
                    && (*tokens.offset((*parser).toksuper as isize)).type_0 as libc::c_uint
                        != JSMN_OBJECT as libc::c_int as libc::c_uint
                {
                    i = (*parser)
                        .toknext
                        .wrapping_sub(1 as libc::c_int as libc::c_uint)
                        as libc::c_int;
                    while i >= 0 as libc::c_int {
                        if (*tokens.offset(i as isize)).type_0 as libc::c_uint
                            == JSMN_ARRAY as libc::c_int as libc::c_uint
                            || (*tokens.offset(i as isize)).type_0 as libc::c_uint
                                == JSMN_OBJECT as libc::c_int as libc::c_uint
                        {
                            if (*tokens.offset(i as isize)).start != -(1 as libc::c_int)
                                && (*tokens.offset(i as isize)).end == -(1 as libc::c_int)
                            {
                                (*parser).toksuper = i;
                                break;
                            }
                        }
                        i -= 1
                    }
                }
            }
            _ => {
                /* In non-strict mode every unquoted value is a primitive */
                r = jsmn_parse_primitive(parser, js, len, tokens, num_tokens as size_t);
                if r < 0 as libc::c_int {
                    return r;
                }
                count += 1;
                if (*parser).toksuper != -(1 as libc::c_int) && !tokens.is_null() {
                    let ref mut fresh2 = (*tokens.offset((*parser).toksuper as isize)).size;
                    *fresh2 += 1
                }
            }
        }
        (*parser).pos = (*parser).pos.wrapping_add(1)
    }
    if !tokens.is_null() {
        i = (*parser)
            .toknext
            .wrapping_sub(1 as libc::c_int as libc::c_uint) as libc::c_int;
        while i >= 0 as libc::c_int {
            /* Unmatched opened object or array */
            if (*tokens.offset(i as isize)).start != -(1 as libc::c_int)
                && (*tokens.offset(i as isize)).end == -(1 as libc::c_int)
            {
                return JSMN_ERROR_PART as libc::c_int;
            }
            i -= 1
        }
    }
    return count;
}
/* JSMN_H */
/* JSMN_HEADER */
