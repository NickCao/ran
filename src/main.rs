use std::io::Read;

fn main() {
    let mut buf = vec![];
    std::io::stdin().read_to_end(&mut buf).unwrap();
    println!("{}", ran::nar(&buf).unwrap().1);
}
