use tree_sitter::{InputEdit, Language, Parser, Point};

fn main() {
    let mut parser = Parser::new();
    println!("Hello, world!");
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(4, 4);
    }
}