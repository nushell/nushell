use std::borrow::Cow;

fn handle_dots_push(string: &mut String, count: u8) {
    if count < 1 {
        return;
    }

    if count == 1 {
        string.push('.');
        return;
    }

    for _ in 0..(count - 1) {
        string.push_str("../");
    }

    string.pop(); // remove last '/'
}

pub fn expand_ndots(path: &str) -> Cow<'_, str> {
    let mut dots_count = 0u8;
    let ndots_present = {
        for chr in path.chars() {
            if chr == '.' {
                dots_count += 1;
            } else {
                if dots_count > 2 {
                    break;
                }

                dots_count = 0;
            }
        }

        dots_count > 2
    };

    if !ndots_present {
        return path.into();
    }

    let mut dots_count = 0u8;
    let mut expanded = String::new();
    for chr in path.chars() {
        if chr != '.' {
            handle_dots_push(&mut expanded, dots_count);
            dots_count = 0;
            expanded.push(chr);
        } else {
            dots_count += 1;
        }
    }

    handle_dots_push(&mut expanded, dots_count);

    expanded.into()
}

pub fn expand_path<'a>(path: &'a str) -> Cow<'a, str> {
    let tilde_expansion: Cow<'a, str> = shellexpand::tilde(path);
    let ndots_expansion: Cow<'a, str> = match tilde_expansion {
        Cow::Borrowed(b) => expand_ndots(b),
        Cow::Owned(o) => expand_ndots(&o).to_string().into(),
    };

    ndots_expansion
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_without_ndots() {
        assert_eq!("../hola", &expand_ndots("../hola").to_string());
    }

    #[test]
    fn string_with_three_ndots() {
        assert_eq!("../..", &expand_ndots("...").to_string());
    }

    #[test]
    fn string_with_three_ndots_and_final_slash() {
        assert_eq!("../../", &expand_ndots(".../").to_string());
    }

    #[test]
    fn string_with_three_ndots_and_garbage() {
        assert_eq!(
            "ls ../../ garbage.*[",
            &expand_ndots("ls .../ garbage.*[").to_string(),
        );
    }
}
