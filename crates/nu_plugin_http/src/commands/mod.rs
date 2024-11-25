pub(super) mod client;
mod delete;
mod get;
mod head;
mod http;
mod options;
mod patch;
mod post;
mod put;

pub use delete::Delete as HttpDelete;
pub use get::Get as HttpGet;
pub use head::Head as HttpHead;
pub use http::Http;
pub use options::Options as HttpOptions;
pub use patch::Patch as HttpPatch;
pub use post::Post as HttpPost;
pub use put::Put as HttpPut;
