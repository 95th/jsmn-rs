use jsmn::*;

fn main() {
    let s = br#"["123", {"a": 1, "b": "c"}, 123]"#;

    let tokens = &mut [Token::default(); 8];
    let p = &mut JsonParser::new();
    let r = p.parse(s, tokens).unwrap();

    println!("Parsed: {:#?}", &tokens[..r as usize]);
}
