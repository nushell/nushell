use nu_pretty_hex::*;

fn main() {
    let config = HexConfig {
        title: true,
        ascii: true,
        width: 16,
        group: 4,
        chunk: 1,
        address_offset: 0,
        skip: Some(10),
        // length: Some(5),
        // length: None,
        length: Some(50),
    };

    let my_string = "Darren Schroeder 😉";
    println!("ConfigHex\n{}\n", config_hex(&my_string, config));
    println!("SimpleHex\n{}\n", simple_hex(&my_string));
    println!("PrettyHex\n{}\n", pretty_hex(&my_string));
    println!("ConfigHex\n{}\n", config_hex(&my_string, config));

    // let mut my_str = String::new();
    // for x in 0..256 {
    //     my_str.push(x as u8);
    // }
    let mut v: Vec<u8> = vec![];
    for x in 0..=127 {
        v.push(x);
    }
    let my_str = String::from_utf8_lossy(&v[..]);

    println!("First128\n{}\n", pretty_hex(&my_str.as_bytes()));
    println!(
        "First128-Param\n{}\n",
        config_hex(&my_str.as_bytes(), config)
    );

    let mut r_str = String::new();
    for _ in 0..=127 {
        r_str.push(rand::random::<u8>() as char);
    }

    println!("Random127\n{}\n", pretty_hex(&r_str));
}

//chunk 0 44617272656e20536368726f65646572   Darren Schroeder
//chunk 1 44 61 72 72  65 6e 20 53  63 68 72 6f  65 64 65 72   Darren Schroeder
//chunk 2 461 7272 656e 2053  6368 726f 6564 6572   Darren Schroeder
//chunk 3 46172 72656e 205363 68726f  656465 72   Darren Schroeder
//chunk 4 44617272 656e2053 6368726f 65646572   Darren Schroeder
