mod client;
mod delete;
mod get;
mod head;
mod http_;
mod options;
mod patch;
mod post;
mod put;
mod timeout_extractor_reader;

pub use delete::HttpDelete;
pub use get::HttpGet;
pub use head::HttpHead;
pub use http_::Http;
pub use options::HttpOptions;
pub use patch::HttpPatch;
pub use post::HttpPost;
pub use put::HttpPut;
