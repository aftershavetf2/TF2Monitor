use std::error::Error;

// From https://stevedonovan.github.io/rust-gentle-intro/6-error-handling.html
// Thanks Steve.
pub type BoxResult<T> = Result<T, Box<dyn Error>>;
