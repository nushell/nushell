mod generic_digest;
mod hash_;
mod md5;
mod sha256;

pub use self::hash_::Hash;
pub use self::md5::HashMd5;
pub use self::sha256::HashSha256;
