use crate::frontend::ast::Parser;
use crate::frontend::lexer::Lexer;
use std::env;
use std::time::Instant;

mod frontend;

fn main() {
    // ======================
    //         Sys
    // ======================

    let mut path = "foc.ctx".to_string();
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

    // ======================
    //        Lexer
    // ======================

    let t0 = Instant::now();

    let mut lexer = Lexer::new(path.clone(), 4096 * 2);
    lexer.lex();

    let elapsed0 = t0.elapsed();
    let n = lexer.tokStream.len();
    if dump {
        println!("\n----------------------------------------");
        println!("Tok-Stream");
        println!("----------------------------------------");
        println!("file: {path}");
        println!("tokens: {n}");
        println!("lex: {:.3?}", elapsed0);
        println!("----------------------------------------");
        for token in &lexer.tokStream {
            println!("{:?}", token.as_ref().unwrap());
        }
    }

    // ======================
    //         AST
    // ======================

    let t1 = Instant::now();

    unsafe {
        let end = (lexer.tokStream.as_ptr()).add(lexer.tokStream.len());
        let mut Parser = Parser::new(lexer.tokStream);
        let ast = Parser.from_ast();

        let elapsed1 = t1.elapsed();
        println!("ast: {:.3?}", elapsed1);
        if dump {
            println!("\n----------------------------------------");
            println!("Abstract Syntax Tree");
            println!("----------------------------------------");
            println!("ast: {:.3?}", elapsed1);
            println!("----------------------------------------");
            println!("{:#?}", ast);
        }
    }
}
