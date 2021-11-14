// #![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;
extern crate nu_pretty_hex;

#[cfg(feature = "alloc")]
use alloc::{format, string::String, vec, vec::Vec};
use nu_pretty_hex::*;

#[cfg(feature = "alloc")]
#[test]
fn test_simple() {
    let bytes: Vec<u8> = (0..16).collect();
    let expected = "00 01 02 03  04 05 06 07  08 09 0a 0b  0c 0d 0e 0f";
    assert_eq!(expected, simple_hex(&bytes));
    assert_eq!(expected, bytes.hex_dump().to_string());
    assert_eq!(simple_hex(&bytes), config_hex(&bytes, HexConfig::simple()));

    let mut have = String::new();
    simple_hex_write(&mut have, &bytes).unwrap();
    assert_eq!(expected, have);

    let str = "string";
    let string: String = String::from("string");
    let slice: &[u8] = &[0x73, 0x74, 0x72, 0x69, 0x6e, 0x67];
    assert_eq!(simple_hex(&str), "73 74 72 69  6e 67");
    assert_eq!(simple_hex(&str), simple_hex(&string));
    assert_eq!(simple_hex(&str), simple_hex(&slice));

    assert!(simple_hex(&vec![]).is_empty());
}

#[cfg(feature = "alloc")]
#[test]
fn test_pretty() {
    let bytes: Vec<u8> = (0..256).map(|x| x as u8).collect();
    let want = include_str!("256.txt");

    let mut hex = String::new();
    pretty_hex_write(&mut hex, &bytes).unwrap();
    assert_eq!(want, hex);
    assert_eq!(want, format!("{:?}", bytes.hex_dump()));
    assert_eq!(want, pretty_hex(&bytes));
    assert_eq!(want, config_hex(&bytes, HexConfig::default()));

    assert_eq!("Length: 0 (0x0) bytes\n", pretty_hex(&vec![]));
}

#[cfg(feature = "alloc")]
#[test]
fn test_config() {
    let cfg = HexConfig {
        title: false,
        ascii: false,
        width: 0,
        group: 0,
        chunk: 0,
    };
    assert!(config_hex(&vec![], cfg).is_empty());
    assert_eq!("2425262728", config_hex(&"$%&'(", cfg));

    let v = include_bytes!("data");
    let cfg = HexConfig {
        title: false,
        group: 8,
        ..HexConfig::default()
    };
    let hex = "0000:   6b 4e 1a c3 af 03 d2 1e  7e 73 ba c8 bd 84 0f 83   kN......~s......\n\
         0010:   89 d5 cf 90 23 67 4b 48  db b1 bc 35 bf ee         ....#gKH...5..";
    assert_eq!(hex, config_hex(&v, cfg));
    assert_eq!(hex, format!("{:?}", v.hex_conf(cfg)));
    let mut str = String::new();
    hex_write(&mut str, v, cfg).unwrap();
    assert_eq!(hex, str);

    assert_eq!(
        config_hex(
            &v,
            HexConfig {
                ascii: false,
                ..cfg
            }
        ),
        "0000:   6b 4e 1a c3 af 03 d2 1e  7e 73 ba c8 bd 84 0f 83\n\
         0010:   89 d5 cf 90 23 67 4b 48  db b1 bc 35 bf ee"
    );

    assert_eq!(
        config_hex(
            &v,
            HexConfig {
                ascii: false,
                group: 4,
                chunk: 2,
                ..cfg
            }
        ),
        "0000:   6b4e 1ac3 af03 d21e  7e73 bac8 bd84 0f83\n\
         0010:   89d5 cf90 2367 4b48  dbb1 bc35 bfee"
    );

    let v: Vec<u8> = (0..21).collect();
    let want = r##"Length: 21 (0x15) bytes
0000:   00 01 02 03  04 05 06 07  08 09 0a 0b  0c 0d 0e 0f   ................
0010:   10 11 12 13  14                                      ....."##;
    assert_eq!(want, pretty_hex(&v));

    let v: Vec<u8> = (0..13).collect();
    assert_eq!(
        config_hex(
            &v,
            HexConfig {
                title: false,
                ascii: true,
                width: 11,
                group: 2,
                chunk: 3
            }
        ),
        "0000:   000102 030405  060708 090a   ...........\n\
         000b:   0b0c                         .."
    );

    let v: Vec<u8> = (0..19).collect();
    assert_eq!(
        config_hex(
            &v,
            HexConfig {
                title: false,
                ascii: true,
                width: 16,
                group: 3,
                chunk: 3
            }
        ),
        "0000:   000102 030405 060708  090a0b 0c0d0e 0f   ................\n\
         0010:   101112                                   ..."
    );

    let cfg = HexConfig {
        title: false,
        group: 0,
        ..HexConfig::default()
    };
    assert_eq!(
        format!("{:?}", v.hex_conf(cfg)),
        "0000:   00 01 02 03 04 05 06 07 08 09 0a 0b 0c 0d 0e 0f   ................\n\
         0010:   10 11 12                                          ..."
    );
    assert_eq!(
        v.hex_conf(cfg).to_string(),
        "00 01 02 03 04 05 06 07 08 09 0a 0b 0c 0d 0e 0f 10 11 12"
    );
}

// This test case checks that hex_write works even without the alloc crate.
// Decorators to this function like simple_hex_write or PrettyHex::hex_dump()
// will be tested when the alloc feature is selected because it feels quite
// cumbersome to set up these tests without the comodity from `alloc`.
#[test]
fn test_hex_write_with_simple_config() {
    let config = HexConfig::simple();
    let bytes = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
    let expected =
        core::str::from_utf8(b"00 01 02 03  04 05 06 07  08 09 0a 0b  0c 0d 0e 0f").unwrap();
    // let expected =
    //     "\u{1b}[38;5;242m00\u{1b}[0m \u{1b}[1;35m01\u{1b}[0m \u{1b}[1;35m02\u{1b}[0m \u{1b}[1;";
    let mut buffer = heapless::Vec::<u8, 50>::new();

    hex_write(&mut buffer, &bytes, config, None).unwrap();

    let have = core::str::from_utf8(&buffer).unwrap();
    assert_eq!(expected, have);
}
