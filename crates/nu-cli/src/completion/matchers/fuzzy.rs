use crate::completion::matchers;

pub struct Matcher;

impl matchers::Matcher for Matcher  {
    fn matches (
        &self,
        partial: &str,
        from: &str
    ) -> bool {
        //FIXME: get complete char list, find out other fuzzy matcher algos
        let ignore_chars = "- _,.,!~ /\\";
        let from_replaced = drop_chars_in_str(from, ignore_chars);
        let partial_replaced = drop_chars_in_str(partial, ignore_chars);
        
        let unicode_matcher: Box<dyn matchers::Matcher> = Box::new(matchers::unicode_case_insensitive::Matcher);
        unicode_matcher.matches(partial_replaced.as_str(), from_replaced.as_str())
    }
}

fn drop_chars_in_str(in_str: &str, drop_chars: &str) -> String {
    in_str
        .split(|ch|drop_chars.contains(ch))
        .collect::<String>()
}