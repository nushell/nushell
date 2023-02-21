mod client;
mod delete;
mod get;
mod http_;
mod post;

pub use delete::SubCommand as HttpDelete;
pub use get::SubCommand as HttpGet;
pub use http_::Http;
pub use post::SubCommand as HttpPost;
