pub mod file;

#[derive(Debug)]
pub enum SymStoreErr {
    NotAFile,
    IOErr(std::io::Error),
}
