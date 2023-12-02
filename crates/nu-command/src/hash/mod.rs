mod generic_digest;
mod hash_;
mod md5;
mod sha256;

pub use self::{hash_::Hash, md5::HashMd5, sha256::HashSha256};
