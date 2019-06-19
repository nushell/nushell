#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::io::prelude::*;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};

    fn test_helper(test_name: &str) {
        let mut baseline_path = PathBuf::new();
        baseline_path.push("tests");
        baseline_path.push(test_name);
        baseline_path.set_extension("out");

        let mut txt_path = PathBuf::new();
        txt_path.push("tests");
        txt_path.push(test_name);
        txt_path.set_extension("txt");

        let executable = {
            let mut buf = PathBuf::new();
            buf.push("target");
            buf.push("debug");
            buf.push("nu");
            buf
        };

        let process = match Command::new(executable)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
        {
            Ok(process) => process,
            Err(why) => panic!("Can't run test {}", why.description()),
        };

        let baseline_out = std::fs::read_to_string(baseline_path).unwrap();
        let baseline_out = baseline_out.replace("\r\n", "\n");
        let input_commands = std::fs::read_to_string(txt_path).unwrap();

        match process.stdin.unwrap().write_all(input_commands.as_bytes()) {
            Err(why) => panic!("couldn't write to wc stdin: {}", why.description()),
            Ok(_) => {}
        }

        let mut s = String::new();
        match process.stdout.unwrap().read_to_string(&mut s) {
            Err(why) => panic!("couldn't read stdout: {}", why.description()),
            Ok(_) => {
                let s = s.replace("\r\n", "\n");
                assert_eq!(s, baseline_out);
            }
        }
    }

    #[test]
    fn open_toml() {
        test_helper("open_toml");
    }

    #[test]
    fn open_json() {
        test_helper("open_json");
    }

    #[test]
    fn open_xml() {
        test_helper("open_xml");
    }

    #[test]
    fn open_ini() {
        test_helper("open_ini");
    }

    #[test]
    fn json_roundtrip() {
        test_helper("json_roundtrip");
    }

    #[test]
    fn toml_roundtrip() {
        test_helper("toml_roundtrip");
    }

    #[test]
    fn sort_by() {
        test_helper("sort_by");
    }

    #[test]
    fn split() {
        test_helper("split");
    }

    #[test]
    fn enter() {
        test_helper("enter");
    }

    #[test]
    fn lines() {
        test_helper("lines");
    }


    #[test]
    fn external_num() {
        test_helper("external_num");
    }
}
