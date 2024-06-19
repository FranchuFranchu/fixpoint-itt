#![feature(box_patterns, debug_closure_helpers)]

pub mod coherence;
pub mod lambda;
pub mod run;
pub mod tree;
pub mod parser;

fn main() {
    let code = std::fs::read_to_string(std::env::args().skip(1).next().unwrap()).unwrap();
    let mut book = match parser::TreeParser::new(&code).parse_book() {
        Ok(o) => o,
        Err(e) => todo!("{}", e),
    };
    for test_name in book.tests {
        let mut test = book.defs.get(&test_name).unwrap().clone();
        test.validate();
        test.normal(|_| ());
        eprintln!("test {test_name}: {}", if test.is_coherent() { "✔️ coherent"} else { "✖️ incoherent"})
    }
    book.root.validate();
    book.root.normal(|x| println!("{}", x.display()));
    book.root.root.resolve_vars(&mut book.root.vars);
    println!("{}", book.root.display());
    println!("Is coherent? {}", book.root.root.is_coherent());
}
