fn main() {
    std::env::args().skip(1).for_each(|arg| print!("{}", arg));
}
