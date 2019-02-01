use log::info;
use std::fs::File;
use std::io::Read;
use walkdir::WalkDir;

fn is_elf_file(path: &std::path::Path) -> bool {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(_) => return false,
    };
    let mut buffer = [0u8; 4];

    match file.read_exact(&mut buffer) {
        Ok(_) => (),
        Err(_) => return false
    };

    buffer[0] == 0x7F && 
    buffer[1] == 'E' as u8 && 
    buffer[2] == 'L' as u8 && 
    buffer[3] == 'F' as u8
}

fn main() {
    let mut builder = pretty_env_logger::formatted_builder();

    let matches = clap::App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about("CLI tool for dbgsrv")
        .arg(clap::Arg::with_name("v")
            .short("v")
            .multiple(true)
            .help("Sets the level of verbosity"))
        .subcommand(clap::SubCommand::with_name("upload")
            .about("Upload the debug info to a debug server")
            .arg(clap::Arg::with_name("PATH")
                .help("Path to search for elf files")
                .required(true)
                .index(1))
            .arg(clap::Arg::with_name("recursive")
                .short("r")
                .long("recursive")
                .help("Search path recursively")))
        .get_matches();

    // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'myprog -v -v -v' or 'myprog -vvv' vs 'myprog -v'
    let builder = match matches.occurrences_of("v") {
        0 => builder.filter_level(log::LevelFilter::Error),
        1 => builder.filter_level(log::LevelFilter::Debug),
        2 => builder.filter_level(log::LevelFilter::Info),
        3 | _ => builder.filter_level(log::LevelFilter::Trace),
    };
    builder.init();

    if let Some(matches) = matches.subcommand_matches("upload") {
        info!("Upload subcommand");
        let path = std::path::Path::new(matches.value_of("PATH").unwrap());
        let max_depth = if matches.is_present("recursive") {
            std::usize::MAX
        } else {
            1
        };

        let elfs = WalkDir::new(path)
            .max_depth(max_depth)
            .into_iter()
            .filter_map(|v| v.ok())
            .filter(|x| is_elf_file(x.path()))
            .collect::<Vec<walkdir::DirEntry>>();

        for dir in &elfs {
            println!("{}", dir.path().display());
        }
    }
}
