mod base64_;
mod command;
mod generic_digest;
mod md5_;
mod sha256_;

pub use base64_::SubCommand as HashBase64;
pub use command::Command as Hash;
pub use md5_::SubCommand as HashMd5;
pub use sha256_::SubCommand as HashSha256;
