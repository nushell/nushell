fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        // Write back out all the arguments passed
        // if given at least 1 instead of chickens
        // speaking co co co.
        let mut arguments = args.iter();
        arguments.next();

        for arg in arguments {
            println!("{}", &arg);
        }
    } else {
        println!("cococo");
    }
}
