use bcodec::*;

fn main() {
    let mut p = JsonParser::new();
    let mut t = vec![Token::default(); 128];
    let s = br#"{"hello": {"hello": {"hello": 12}}}"#;
    let r = p.parse(s, &mut t).unwrap();

    println!("Parsed: {:#?}", &t[..r as usize]);
}
