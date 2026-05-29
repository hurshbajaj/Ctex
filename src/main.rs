use crate::frontend::lexer::Lexer;

mod frontend;

fn main() {
    let mut Lexer = Lexer::new(String::from("dummy.ctx"));
    Lexer.lex();
    println!("{:?}", Lexer.tokStream);
}
