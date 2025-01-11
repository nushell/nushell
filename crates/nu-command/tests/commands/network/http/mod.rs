mod delete;
mod get;
mod head;
mod options;
mod patch;
mod post;
mod put;

/// String representation of the Windows error code for timeouts on slow links.
///
/// Use this constant in tests instead of matching partial error message content,
/// such as `"did not properly respond after a period of time"`, which can vary by language.
/// The specific string `"(os error 10060)"` is consistent across all locales, as it represents
/// the raw error code rather than localized text.
///
/// For more details, see the [Microsoft docs](https://learn.microsoft.com/en-us/troubleshoot/windows-client/networking/10060-connection-timed-out-with-proxy-server).
#[cfg(all(test, windows))]
const WINDOWS_ERROR_TIMEOUT_SLOW_LINK: &str = "(os error 10060)";
