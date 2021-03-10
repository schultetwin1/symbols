use log::{/*error,*/ /*debug,*/ info, trace, warn};
use std::fs::File;
use std::io::Read;

use symbolic_debuginfo::Object;

use crate::symstore::SymStoreErr;

pub fn file_to_key(path: &std::path::Path) -> Result<Option<std::string::String>, SymStoreErr> {
    trace!("Inspecting file {}", path.display());
    if !path.is_file() {
        return Err(SymStoreErr::NotAFile);
    }

    let filename = path.file_name().unwrap().to_str().unwrap();

    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(err) => {
            warn!("Unable to open file {}", path.display());
            warn!("Error: {}", err);
            return Err(SymStoreErr::IOErr(err));
        }
    };

    let mut buffer = Vec::new();

    match file.read_to_end(&mut buffer) {
        Ok(_) => (),
        Err(err) => {
            warn!("Unable to read to end of file {}", path.display());
            warn!("Error: {}", err);
            return Err(SymStoreErr::IOErr(err));
        }
    };

    let result = match Object::parse(&buffer) {
        Ok(obj) => object_to_key(&filename, &obj),
        Err(_err) => {
            info!("Failed to parse file {}", path.display());
            Ok(None)
        }
    };

    drop(buffer);
    result
}

fn object_to_key(filename: &str, obj: &Object) -> Result<Option<std::string::String>, SymStoreErr> {
    match obj {
        Object::Pe(pe) => pe_to_key(filename, &pe),
        Object::Pdb(pdb) => pdb_to_key(filename, &pdb),
        Object::Elf(elf) => elf_to_key(&elf),
        Object::MachO(macho) => macho_to_key(filename, &macho),
        _ => Ok(None),
    }
}

fn pe_to_key(
    filename: &str,
    pe: &symbolic_debuginfo::pe::PeObject,
) -> Result<Option<std::string::String>, SymStoreErr> {
    if let Some(code_id) = pe.code_id() {
        let key = format!(
            "{filename}/{codeid}/{filename}",
            filename = filename,
            codeid = code_id.as_str()
        );
        Ok(Some(key))
    } else {
        Ok(None)
    }
}

fn pdb_to_key(
    filename: &str,
    pdb: &symbolic_debuginfo::pdb::PdbObject,
) -> Result<Option<std::string::String>, SymStoreErr> {
    let key = format!(
        "{filename}/{sig:X}{age:X}/{filename}",
        filename = filename,
        sig = pdb.debug_id().uuid().to_simple_ref(),
        age = pdb.debug_id().appendix()
    );
    Ok(Some(key))
}

fn elf_to_key(
    elf: &symbolic_debuginfo::elf::ElfObject,
) -> Result<Option<std::string::String>, SymStoreErr> {
    if let Some(code_id) = elf.code_id() {
        let key = if elf.has_debug_info() {
            format!("buildid/{note}/debuginfo", note = code_id.as_ref())
        } else {
            format!("buildid/{note}/executable", note = code_id.as_ref())
        };
        Ok(Some(key))
    } else {
        Ok(None)
    }
}

fn macho_to_key(
    filename: &str,
    macho: &symbolic_debuginfo::macho::MachObject,
) -> Result<Option<std::string::String>, SymStoreErr> {
    if let Some(code_id) = macho.code_id() {
        let key = if macho.has_debug_info() {
            format!(
                "{filename}/mach-uuid-{note}/{filename}",
                filename = filename,
                note = code_id.as_ref()
            )
        } else {
            format!(
                "_.dwarf/mach-uuid-sym{note}/_.dwarf",
                note = code_id.as_ref()
            )
        };
        Ok(Some(key))
    } else {
        Ok(None)
    }
}
