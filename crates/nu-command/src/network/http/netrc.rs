//! Taken from
//! https://github.com/ducaale/xh/blob/f50f10c87c1c92c487a57561a8a1ed64eb5ed5ab/src/netrc.rs and
//! modified for nushells needs.
//!
//! See https://www.gnu.org/software/inetutils/manual/html_node/The-_002enetrc-file.html
//!
//! And https://github.com/curl/curl/blob/b01165680450364bdc770da3c7ede190872286c8/lib/netrc.c
//!
//! HTTPie has this behavior:
//!
//! - Entries must have both a login and a password or they'll be ignored or misbehave.
//!
//! - Fields from a default entry are not mixed with those of another entry.
//!
//! - An incomplete entry doesn't allow the default entry as a fallback.
//!
//! - The default entry doesn't have to be at the end of the file.
//!
//! HTTPie uses the implementation from Python's standard library
//! (with a wrapper from requests).
//!
//! This implementation is not at all strict, files are never rejected outright.
//! We'd ignore errors anyway to match HTTPie so that might be for the best.
//! (HTTPie's parser is strict, so a minor problem will silently stop the file
//! from being used.)
//!
//! This implementation additionally handles entries with just a password and no login,
//! to support using .netrc for bearer auth.
//!
//! This is too specialized for our use case to be a crate, but feel free to
//! copy/paste into another project and modify.

use std::{
    fs::File,
    io::{self, BufRead, BufReader},
    path::PathBuf,
};

use nu_path::home_dir;

#[derive(Debug, PartialEq, Eq)]
pub struct Entry {
    pub login: Option<String>,
    pub password: String,
}

pub fn find_entry(host: url::Host<&str>, path: NetrcPath) -> Option<Entry> {
    let file = open_netrc(&path)?;

    let file = BufReader::new(file);
    let parser = Parser::new(file, host);

    // Logging I/O errors would be nice.
    parser.parse().ok()?
}

#[derive(Debug, PartialEq, Eq)]
pub enum NetrcPath {
    Default,
    FromEnv(PathBuf),
}

fn open_netrc(path: &NetrcPath) -> Option<File> {
    match path {
        NetrcPath::Default => {
            let home_dir = home_dir()?;

            [".netrc", "_netrc"]
                .iter()
                .map(|path| home_dir.join(path))
                .find_map(|path| File::open(path).ok())
        }
        NetrcPath::FromEnv(path) => File::open(path).ok(),
    }
}

#[derive(Copy, Clone)]
enum EntryState {
    /// We're outside any entry, or in one for the wrong host.
    Wrong,
    /// We're inside the entry for the host we want.
    Correct,
    /// We're inside the default entry.
    Default,
}

struct Parser<'a, R> {
    reader: R,
    /// The current line.
    buf: String,
    /// The index in `buf` to start looking for the next word.
    pos: usize,
    /// The host we're looking for.
    host: url::Host<&'a str>,
    /// Info about the entry we're handling.
    state: EntryState,
    /// The data collected for the current entry.
    login: Option<String>,
    password: Option<String>,
    account: Option<String>,
    /// Whether to block the default entry from being returned.
    suppress_default: bool,
    /// The default entry, to return if no other can be found.
    default: Option<Entry>,
    /// A complete relevant entry, to be returned ASAP.
    entry: Option<Entry>,
}

impl<'a, R: BufRead> Parser<'a, R> {
    fn new(reader: R, host: url::Host<&'a str>) -> Self {
        Parser {
            reader,
            buf: String::new(),
            pos: 0,
            host,
            state: EntryState::Wrong,
            login: None,
            password: None,
            account: None,
            suppress_default: false,
            default: None,
            entry: None,
        }
    }

    fn parse(mut self) -> io::Result<Option<Entry>> {
        while let Some(word) = self.word()? {
            // curl does a case-insensitive comparison here but that
            // seems unnecessary.
            match word {
                "default" => {
                    // The default entry. Some implementations want you to put it at the
                    // end of the file so they can unconditionally stop after finding it,
                    // we'll use it as a true fallback (like Python does).
                    self.finish_entry();
                    self.state = EntryState::Default;
                }
                "machine" => {
                    self.finish_entry();
                    if let Some(new_host) = self.word()? {
                        match url::Host::parse(new_host) {
                            Ok(new_host) if self.host == new_host => {
                                self.state = EntryState::Correct;
                                self.suppress_default = true;
                            }
                            _ => {
                                self.state = EntryState::Wrong;
                            }
                        }
                    }
                }
                "login" => {
                    if let Some(login) = self.arg()? {
                        self.login = Some(login);
                    }
                }
                "password" => {
                    if let Some(password) = self.arg()? {
                        // Some implementations check the permissions of the file here.
                        // It should be owned by the current user and not be readable by
                        // anyone else. (Unless it contains no passwords.)
                        // But that's a lot of work and somewhat less vital in the
                        // single-user age. Python's stdlib does it by default, but
                        // requests/HTTPie avoids that check.
                        self.password = Some(password);
                    }
                }
                "account" => {
                    // requests/HTTPie uses this as a fallback for login.
                    if let Some(account) = self.arg()? {
                        self.account = Some(account);
                    }
                }
                "macdef" => {
                    // Macro definition. We ignore these.
                    self.finish_entry();
                    // Consume the macro's name.
                    self.word()?;
                    // Skip until the next blank line.
                    // (We consider a line with just whitespace blank.)
                    self.advance_line()?;
                    while !self.buf.trim().is_empty() {
                        self.advance_line()?;
                    }
                }
                word if word.starts_with('#') => {
                    // Comment, skip the rest of the line.
                    // By doing the check here instead of in Reader::word() we allow
                    // arguments to machine/login/password/account to start with #. Curl
                    // doesn't do this.
                    // Python supports comments but seems to dislike blank lines inbetween
                    // commented lines.
                    self.advance_line()?;
                }
                _ => {
                    // Unknown word. We don't crash, but do consider this the end
                    // of the entry.
                    self.finish_entry();
                }
            }
            if let Some(entry) = self.entry {
                return Ok(Some(entry));
            }
        }
        self.finish_entry();
        if let Some(entry) = self.entry {
            Ok(Some(entry))
        } else if self.suppress_default {
            Ok(None)
        } else {
            Ok(self.default)
        }
    }

    /// Reset the current entry state. Try to build an entry out of what was gathered.
    fn finish_entry(&mut self) {
        let login = self.login.take();
        let account = self.account.take();
        let password = self.password.take();

        let state = self.state;
        self.state = EntryState::Wrong;

        if let (login, Some(password)) = (login.or(account), password) {
            let entry = Entry { login, password };
            match state {
                EntryState::Wrong => unreachable!("netrc: Should not have been storing info"),
                EntryState::Correct => self.entry = Some(entry),
                EntryState::Default => self.default = Some(entry),
            }
        }
    }

    /// Consume the next word. Return it only if we're processing a relevant entry.
    fn arg(&mut self) -> io::Result<Option<String>> {
        let state = self.state;
        let word = self.word()?;
        match state {
            EntryState::Wrong => Ok(None),
            EntryState::Correct | EntryState::Default => Ok(word.map(str::to_owned)),
        }
    }

    /// Advance the reader/buffer to the next line.
    fn advance_line(&mut self) -> io::Result<usize> {
        self.buf.clear();
        self.pos = 0;
        self.reader.read_line(&mut self.buf)
    }

    /// Read the next word, if any.
    fn word(&mut self) -> io::Result<Option<&str>> {
        loop {
            match self.buf[self.pos..].chars().next() {
                Some(ch) if ch.is_whitespace() => self.pos += ch.len_utf8(),
                Some(_) => match self.buf[self.pos..].split_whitespace().next() {
                    Some(text) => {
                        self.pos += text.len();
                        break Ok(Some(text));
                    }
                    None => break Ok(None),
                },
                None => {
                    if self.advance_line()? == 0 {
                        break Ok(None);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::net::Ipv4Addr;

    const COM: url::Host<&str> = url::Host::Domain("example.com");
    const ORG: url::Host<&str> = url::Host::Domain("example.org");
    const UNI: url::Host<&str> = url::Host::Domain("xn--9ca.com");
    const IP1: url::Host<&str> = url::Host::Ipv4(Ipv4Addr::new(1, 1, 1, 1));
    const IP2: url::Host<&str> = url::Host::Ipv4(Ipv4Addr::new(2, 2, 2, 2));

    #[test]
    fn parse_simple_config() {
        const SIMPLE: &str = "
            machine example.com
            login user
            password pass
        ";
        found(SIMPLE, COM, "user", "pass");
        notfound(SIMPLE, ORG);
        notfound(SIMPLE, UNI);
        notfound(SIMPLE, IP1);
    }

    #[test]
    fn parse_oneliner_config() {
        const ONELINE: &str = "
            machine example.com login user password pass
        ";
        found(ONELINE, COM, "user", "pass");
        notfound(ONELINE, ORG);
    }

    #[test]
    fn parse_config_with_multiple_records() {
        const MULTI: &str = "
            machine example.com login user password pass
            machine example.org login foo password bar
        ";
        found(MULTI, COM, "user", "pass");
        found(MULTI, ORG, "foo", "bar");
        notfound(MULTI, UNI);
    }

    #[test]
    fn parse_config_with_unicode() {
        const UNICODE: &str = "
            machine É.com login user password pass
        ";
        found(UNICODE, UNI, "user", "pass");
        notfound(UNICODE, COM);
    }

    #[test]
    fn parse_missing_password() {
        const MISSING_PASS: &str = "
            machine example.com login user
        ";
        notfound(MISSING_PASS, COM);
    }

    #[test]
    fn parse_missing_user() {
        const MISSING_USER: &str = "
            machine example.com password pass
            default login user
        ";
        found(MISSING_USER, COM, None, "pass");
        notfound(MISSING_USER, ORG);
    }

    #[test]
    fn parse_missing_default_missing_user() {
        const DEFAULT_PASSWORD_MISSING_USER: &str = "
            machine example.com password pass
            default password def
        ";
        found(DEFAULT_PASSWORD_MISSING_USER, COM, None, "pass");
        found(DEFAULT_PASSWORD_MISSING_USER, ORG, None, "def");
    }

    #[test]
    fn parse_default_last() {
        const DEFAULT_LAST: &str = "
            machine example.com login ex password am
            default login def password ault
        ";
        found(DEFAULT_LAST, COM, "ex", "am");
        found(DEFAULT_LAST, ORG, "def", "ault");
    }

    #[test]
    fn parse_default_first() {
        const DEFAULT_FIRST: &str = "
            default login def password ault
            machine example.com login ex password am
        ";
        found(DEFAULT_FIRST, COM, "ex", "am");
        found(DEFAULT_FIRST, ORG, "def", "ault");
    }

    #[test]
    fn parse_fallback_to_account() {
        const ACCOUNT_FALLBACK: &str = "
            machine example.com account acc password pass
        ";
        found(ACCOUNT_FALLBACK, COM, "acc", "pass");
    }

    #[test]
    fn parse_ignore_account() {
        const ACCOUNT_NOT_PREFERRED: &str = "
            machine example.com password pass login log account acc
            machine example.org password pass account acc login log
        ";
        found(ACCOUNT_NOT_PREFERRED, COM, "log", "pass");
        found(ACCOUNT_NOT_PREFERRED, ORG, "log", "pass");
    }

    #[test]
    fn parse_with_ip() {
        const WITH_IP: &str = "
            machine 1.1.1.1 login us password pa
        ";
        found(WITH_IP, IP1, "us", "pa");
        notfound(WITH_IP, IP2);
        notfound(WITH_IP, COM);
    }

    #[test]
    fn parse_weird_ip() {
        const WEIRD_IP: &str = "
            machine 16843009 login us password pa
        ";
        found(WEIRD_IP, IP1, "us", "pa");
        notfound(WEIRD_IP, IP2);
        notfound(WEIRD_IP, COM);
    }

    #[test]
    fn parse_malformed_config() {
        const MALFORMED: &str = "
            I'm a malformed netrc!
        ";
        notfound(MALFORMED, COM);
    }

    #[test]
    fn parse_ignore_config() {
        const COMMENT: &str = "
            # machine example.com login user password pass
            machine example.org login lo password pa
        ";
        notfound(COMMENT, COM);
        found(COMMENT, ORG, "lo", "pa");
    }

    #[test]
    fn parse_octothorpe() {
        const OCTOTHORPE_IN_VALUE: &str = "
            machine example.com login #!@$ password pass
        ";
        found(OCTOTHORPE_IN_VALUE, COM, "#!@$", "pass");
    }

    #[test]
    fn parse_sudden_end() {
        const SUDDEN_END: &str = "
            machine example.com login
        ";
        notfound(SUDDEN_END, COM);
    }

    #[test]
    fn parse_incomplete_and_default() {
        const INCOMPLETE_AND_DEFAULT: &str = "
            machine example.com login user
            default login u password p
        ";
        notfound(INCOMPLETE_AND_DEFAULT, COM);
        found(INCOMPLETE_AND_DEFAULT, ORG, "u", "p");
    }

    #[test]
    fn parse_unknown_token() {
        const UNKNOWN_TOKEN_INTERRUPT: &str = "
            machine example.com
            login user
            foo bar
            password pass
        ";
        notfound(UNKNOWN_TOKEN_INTERRUPT, COM);
    }

    #[test]
    fn parse_macro() {
        const MACRO: &str = "
            macdef foo
            machine example.com login mac password def
            qux

            machine example.com login user password pass
        ";
        found(MACRO, COM, "user", "pass");
        notfound(MACRO, ORG);
    }

    #[test]
    fn parse_unterminated_macro() {
        const MACRO_UNTERMINATED: &str = "
            macdef foo
            machine example.com login mac password def
            qux
            machine example.com login user password pass";
        notfound(MACRO_UNTERMINATED, COM);
    }

    #[test]
    fn parse_macro_blank_line_before_name() {
        const MACRO_BLANK_LINE_BEFORE_NAME: &str = "
            macdef

            foo
            machine example.com login mac password def";
        notfound(MACRO_BLANK_LINE_BEFORE_NAME, COM);
    }

    #[test]
    fn parse_many_lines() {
        const MANY_LINES: &str = "
            machine
            example.com
            login

            user
            password
            pass
        ";
        found(MANY_LINES, COM, "user", "pass");
    }

    #[test]
    fn parse_strange_characters() {
        const STRANGE_CHARACTERS: &str = "
            machine\u{2029}oké\t\u{2029}login  u   password  p\t\t\t\r\n
        ";
        notfound(STRANGE_CHARACTERS, COM);
    }

    #[track_caller]
    fn found(
        netrc: &str,
        host: url::Host<&str>,
        login: impl Into<Option<&'static str>>,
        password: &str,
    ) {
        let entry = Parser::new(netrc.as_bytes(), host).parse().unwrap();
        let entry = entry.expect("Didn't find entry");
        assert_eq!(entry.login.as_deref(), login.into());
        assert_eq!(entry.password, password);
    }

    #[track_caller]
    fn notfound(netrc: &str, host: url::Host<&str>) {
        let entry = Parser::new(netrc.as_bytes(), host).parse().unwrap();
        assert!(entry.is_none(), "Found entry");
    }
}
