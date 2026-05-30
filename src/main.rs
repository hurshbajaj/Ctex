use crate::frontend::lexer::Lexer;
use std::env;
use std::time::Instant;

mod frontend;

fn main() {
    let mut path = "dummy_large.ctx".to_string();
    let mut dump = false;
    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--dump" => dump = true,
            other if other.starts_with('-') => {
                eprintln!("unknown flag: {other}");
                std::process::exit(1);
            }
            other => path = other.into(),
        }
    }
    let t0 = Instant::now();
    let mut lexer = Lexer::new(path.clone());
    lexer.lex();
    let elapsed = t0.elapsed();
    let n = lexer.tokStream.len();
    println!("file: {path}");
    println!("tokens: {n}");
    println!("lex: {:.3?}", elapsed);
    if dump {
        println!("{:?}", lexer.tokStream);
    }
}
