use hamcrest2::core::{MatchResult, Matcher};
use std::fmt;
use std::str;

use super::nu_process::Outcome;
use super::{Director, Executable};

#[derive(Clone)]
pub struct Play {
    stdout_expectation: Option<String>,
}

impl fmt::Display for Play {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "play")
    }
}

impl fmt::Debug for Play {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "play")
    }
}

pub fn says() -> Play {
    Play {
        stdout_expectation: None,
    }
}

trait CheckerMatchers {
    fn output(&self, actual: &Outcome) -> MatchResult;
    fn std(&self, actual: &[u8], expected: Option<&String>, description: &str) -> MatchResult;
    fn stdout(&self, actual: &Outcome) -> MatchResult;
}

impl CheckerMatchers for Play {
    fn output(&self, actual: &Outcome) -> MatchResult {
        self.stdout(actual)
    }

    fn stdout(&self, actual: &Outcome) -> MatchResult {
        self.std(&actual.out, self.stdout_expectation.as_ref(), "stdout")
    }

    fn std(&self, actual: &[u8], expected: Option<&String>, description: &str) -> MatchResult {
        let out = match expected {
            Some(out) => out,
            None => return Ok(()),
        };
        let actual = match str::from_utf8(actual) {
            Err(..) => return Err(format!("{} was not utf8 encoded", description)),
            Ok(actual) => actual,
        };

        if actual != *out {
            return Err(format!(
                "not equal:\n    actual: {}\n  expected: {}\n\n",
                actual, out
            ));
        }

        Ok(())
    }
}

impl Matcher<Outcome> for Play {
    fn matches(&self, output: Outcome) -> MatchResult {
        self.output(&output)
    }
}

impl Matcher<Director> for Play {
    fn matches(&self, mut director: Director) -> MatchResult {
        self.matches(&mut director)
    }
}

impl<'a> Matcher<&'a mut Director> for Play {
    fn matches(&self, director: &'a mut Director) -> MatchResult {
        if director.executable().is_none() {
            return Err(format!("no such process {}", director));
        }

        let res = director.execute();

        match res {
            Ok(out) => self.output(&out),
            Err(err) => {
                if let Some(out) = &err.output {
                    return self.output(out);
                }

                Err(format!("could not exec process {}: {:?}", director, err))
            }
        }
    }
}

impl Play {
    pub fn stdout(mut self, expected: &str) -> Self {
        self.stdout_expectation = Some(expected.to_string());
        self
    }
}
