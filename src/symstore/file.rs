use log::{/*error,*/ /*debug,*/ info, trace, warn};
use serde::Serialize;
use std::convert::TryInto;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use symbolic_debuginfo::Object;

use crate::symstore::SymStoreErr;

#[derive(Clone, Copy, Eq, Hash, PartialEq, Serialize)]
pub enum FileType {
    Pe,
    Pdb,
    Elf,
    MachO,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ResourceType {
    Executable,
    DebugInfo,
}

#[derive(Eq, Hash, PartialEq)]
pub struct FileInfo {
    pub path: PathBuf,
    pub file_type: FileType,
    pub file_size: usize,
    pub identifier: String,
    pub resource_type: ResourceType,
}

impl FileInfo {
    pub fn key(&self) -> String {
        match self.file_type {
            FileType::Elf => match self.resource_type {
                ResourceType::Executable => {
                    format!("buildid/{}/executable", self.identifier)
                }
                ResourceType::DebugInfo => {
                    format!("buildid/{}/debuginfo", self.identifier)
                }
            },
            FileType::MachO => match self.resource_type {
                ResourceType::Executable => {
                    format!(
                        "{filename}/mach-uuid-{note}/{filename}",
                        filename = self.path.file_name().unwrap().to_str().unwrap(),
                        note = self.identifier
                    )
                }
                ResourceType::DebugInfo => {
                    format!(
                        "_.dwarf/mach-uuid-sym-{note}/_.dwarf",
                        note = self.identifier
                    )
                }
            },
            FileType::Pdb | FileType::Pe => {
                format!(
                    "{filename}/{identifier}/{filename}",
                    filename = self.path.file_name().unwrap().to_str().unwrap(),
                    identifier = self.identifier
                )
            }
        }
    }
}

pub fn file_to_info(path: &std::path::Path) -> Result<Option<FileInfo>, SymStoreErr> {
    trace!("Inspecting file {}", path.display());
    if !path.is_file() {
        return Err(SymStoreErr::NotAFile);
    }

    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(err) => {
            warn!("Unable to open file {}", path.display());
            warn!("Error: {}", err);
            return Err(SymStoreErr::IoErr(err));
        }
    };

    let filesize: usize = file.metadata().unwrap().len().try_into().unwrap();

    let mut buffer = Vec::with_capacity(filesize);

    match file.read_to_end(&mut buffer) {
        Ok(_) => (),
        Err(err) => {
            warn!("Unable to read to end of file {}", path.display());
            warn!("Error: {}", err);
            return Err(SymStoreErr::IoErr(err));
        }
    };

    let result = match Object::parse(&buffer) {
        Ok(obj) => object_to_info(path, filesize, &obj),
        Err(_err) => {
            info!("Failed to parse file {}", path.display());
            None
        }
    };

    Ok(result)
}

fn object_to_info(path: &Path, filesize: usize, obj: &Object) -> Option<FileInfo> {
    match obj {
        Object::Pe(pe) => pe_to_info(path, filesize, pe),
        Object::Pdb(pdb) => Some(pdb_to_info(path, filesize, pdb)),
        Object::Elf(elf) => elf_to_info(path, filesize, elf),
        Object::MachO(macho) => macho_to_info(path, filesize, macho),
        _ => None,
    }
}

fn pe_to_info(
    path: &Path,
    filesize: usize,
    pe: &symbolic_debuginfo::pe::PeObject,
) -> Option<FileInfo> {
    pe.code_id().map(|code_id| FileInfo {
        path: path.to_path_buf(),
        file_type: FileType::Pe,
        file_size: filesize,
        identifier: code_id.to_string(),
        resource_type: ResourceType::Executable,
    })
}

fn pdb_to_info(path: &Path, filesize: usize, pdb: &symbolic_debuginfo::pdb::PdbObject) -> FileInfo {
    let id = format!(
        "{sig:X}{age:X}",
        sig = pdb.debug_id().uuid().to_simple_ref(),
        age = pdb.debug_id().appendix()
    );
    FileInfo {
        path: path.to_path_buf(),
        file_type: FileType::Pdb,
        file_size: filesize,
        identifier: id,
        resource_type: ResourceType::DebugInfo,
    }
}

fn elf_to_info(
    path: &Path,
    filesize: usize,
    elf: &symbolic_debuginfo::elf::ElfObject,
) -> Option<FileInfo> {
    if let Some(code_id) = elf.code_id() {
        if elf.has_debug_info() {
            Some(FileInfo {
                path: path.to_path_buf(),
                file_type: FileType::Elf,
                file_size: filesize,
                identifier: code_id.to_string(),
                resource_type: ResourceType::DebugInfo,
            })
        } else {
            Some(FileInfo {
                path: path.to_path_buf(),
                file_type: FileType::Elf,
                file_size: filesize,
                identifier: code_id.to_string(),
                resource_type: ResourceType::Executable,
            })
        }
    } else {
        None
    }
}

fn macho_to_info(
    path: &Path,
    filesize: usize,
    macho: &symbolic_debuginfo::macho::MachObject,
) -> Option<FileInfo> {
    if let Some(code_id) = macho.code_id() {
        if macho.has_debug_info() {
            Some(FileInfo {
                path: path.to_path_buf(),
                file_type: FileType::MachO,
                file_size: filesize,
                identifier: code_id.to_string(),
                resource_type: ResourceType::DebugInfo,
            })
        } else {
            Some(FileInfo {
                path: path.to_path_buf(),
                file_type: FileType::MachO,
                file_size: filesize,
                identifier: code_id.to_string(),
                resource_type: ResourceType::Executable,
            })
        }
    } else {
        None
    }
}
