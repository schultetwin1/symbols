pub mod file;

#[derive(Debug)]
pub enum SymStoreErr {
    NotAFile,
    IoErr(std::io::Error),
}
