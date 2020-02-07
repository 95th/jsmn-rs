use jsmn::*;

fn main() {
    let s = br#"{"hello": {"hello": {"hello": 12}}}"#;

    let tokens = &mut [Token::default(); 128];
    let p = &mut JsonParser::new();
    let r = p.parse(s, tokens).unwrap();

    println!("Parsed: {:#?}", &tokens[..r as usize]);
}
