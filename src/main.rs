use bcodec::*;

fn main() {
    let mut p = JsmnParser::new();
    let mut t = vec![JsmnToken::default(); 128];
    let s = br#"{"hello": 12}"#;
    let r = jsmn_parse(&mut p, s, &mut t).unwrap();

    println!("Parsed: {:#?}", &t[..r as usize]);
}
