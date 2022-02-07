#[cfg(feature = "fetch")]
mod fetch;
#[cfg(feature = "fetch")]
pub use fetch::Command as Fetch;

#[cfg(feature = "post")]
mod post;
#[cfg(feature = "post")]
pub use post::Command as Post;

mod url_;
pub use url_::*;
