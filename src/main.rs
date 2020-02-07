use bcodec::*;

fn main() {
    let mut p = JsmnParser::new();
    let mut t = vec![JsmnToken::default(); 128];
    let s = br#"{"hello": 12}"#;
    let r = unsafe { jsmn_parse((&mut p) as _, s, (&mut t[0]) as _, 128).unwrap() };

    println!("Parsed: {:#?}", &t[..r as usize]);
}
