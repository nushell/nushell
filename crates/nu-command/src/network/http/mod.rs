mod client;
mod delete;
mod get;
mod head;
mod http_;
mod post;
mod put;

pub use delete::SubCommand as HttpDelete;
pub use get::SubCommand as HttpGet;
pub use head::SubCommand as HttpHead;
pub use http_::Http;
pub use post::SubCommand as HttpPost;
pub use put::SubCommand as HttpPut;
