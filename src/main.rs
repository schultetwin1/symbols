use log::{trace, /*debug,*/ info, warn, error};
use std::fs::File;
use std::io::Read;
use walkdir::WalkDir;

const APP_AUTHOR: &str = "dbgsrv";
const APP_NAME: &str = "dbg";

fn main() {
    let matches = clap::App::new(APP_NAME)
        .version(env!("CARGO_PKG_VERSION"))
        .about("CLI tool for dbgsrv")
        .author(APP_AUTHOR)
        .arg(clap::Arg::with_name("v")
            .short("v")
            .multiple(true)
            .help("Sets the level of verbosity"))
        .subcommand(clap::SubCommand::with_name("upload")
            .about("Upload the debug info files to a debug server")
            .arg(clap::Arg::with_name("PATH")
                .help("Path to search for debug info files")
                .required(true)
                .index(1))
            .arg(clap::Arg::with_name("recursive")
                .short("r")
                .long("recursive")
                .help("Search path recursively")))
        .get_matches();

    initialize_logger(&matches);
    trace!("logger initialized");

    if let Some(matches) = matches.subcommand_matches("upload") {
        info!("Upload subcommand");
        upload_dbg_info(matches);
    }
}

fn initialize_logger(matches: &clap::ArgMatches) {
    // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'myprog -v -v -v' or 'myprog -vvv' vs 'myprog -v'
    let mut logger = pretty_env_logger::formatted_builder();
    let logger = match matches.occurrences_of("v") {
        0 => logger.filter_level(log::LevelFilter::Error),
        1 => logger.filter_level(log::LevelFilter::Warn),
        2 => logger.filter_level(log::LevelFilter::Info),
        3 => logger.filter_level(log::LevelFilter::Debug),
        4 | _ => logger.filter_level(log::LevelFilter::Trace),
    };
    logger.init();
}

fn upload_dbg_info(matches: &clap::ArgMatches) {
    let path = std::path::Path::new(matches.value_of("PATH").unwrap());

    if !path.exists() {
        println!("Path \"{}\" does not exist", path.to_str().unwrap_or("*INVALID PATH*"));
        return;
    }

    let max_depth = if matches.is_present("recursive") {
        std::usize::MAX
    } else {
        1
    };

    let files = WalkDir::new(path)
        .max_depth(max_depth)
        .into_iter()
        .filter_map(|v| v.ok())
        .filter(|x| x.path().is_file())
        .filter(|x| is_debug_info_file(x.path()))
        .collect::<Vec<walkdir::DirEntry>>();

    // Files to upload
    for file in &files {
        println!("uploading {}", file.path().display());
        let mut file = File::open(file.path()).unwrap();

        let mut buffer = Vec::new();

        file.read_to_end(&mut buffer).unwrap();

        let mut response = reqwest::Client::new()
            .post("http://localhost:7071/api/upload")
            .body(buffer)
            .header("Content-Type", "application/octet-stream")
            .send()
            .unwrap();

        println!("{}", response.text().unwrap());

    }
}

fn is_debug_info_file(path: &std::path::Path) -> bool {
    trace!("Inspecting file {}", path.display());
    let file = match File::open(path) {
        Ok(file) => file,
        Err(err) => {
            warn!("Unable to open file {}", path.display());
            warn!("Error: {}", err);
            return false;
        },
    };

    // Test if its a PDB
    if let Ok(mut pdb) = pdb::PDB::open(file) {
        match pdb.pdb_information() {
            Ok(_pdb_info) => return true,
            Err(err) => {
                error!("Unable to read pdb info from {}", path.display());
                error!("Error {}", err);
            }
        }
    }

    let mut file = File::open(path).unwrap();

    let mut buffer = Vec::new();

    match file.read_to_end(&mut buffer) {
        Ok(_) => (),
        Err(e) => {
            warn!("Unable to read to end of file {}", path.display());
            warn!("Error: {}", e);
            return false;
        }
    };

    match goblin::Object::parse(&buffer) {
        Ok(object) => match object {
            goblin::Object::Elf(elf) => return elf_has_buildid(&elf, &buffer),
            goblin::Object::PE(pe) => return pe_has_pdb_info(&pe),
            goblin::Object::Mach(mach) => return mach_has_uuid(&mach),
            goblin::Object::Archive(_archive) => return false,
            goblin::Object::Unknown(_magic) => return false
        },
        Err(_) => return false
    };
}

fn elf_has_buildid(elf: &goblin::elf::Elf, data: &[u8]) -> bool {
    if let Some(notes) = elf.iter_note_headers(data) {
        for note in notes {
            if let Ok(note) = note {
                if note.n_type == goblin::elf::note::NT_GNU_BUILD_ID {
                    return true;
                }
            }
        }
    }
    false
}

fn pe_has_pdb_info(pe: &goblin::pe::PE) -> bool {
    if let Some(debug_data) = pe.debug_data {
        if let Some(_debug_info) = debug_data.codeview_pdb70_debug_info {
            return true;
        }
    }
    false
}

fn mach_has_uuid(mach: &goblin::mach::Mach) -> bool {
    match mach {
        // Currently fat arch are not supported
        goblin::mach::Mach::Fat(_multiarch) => return false,
        goblin::mach::Mach::Binary(macho) => {
            return macho.load_commands.iter().find(|&x| match x.command {
                goblin::mach::load_command::CommandVariant::Uuid(_) => true,
                _ => false,
            }).is_some();
        }
    };
}
