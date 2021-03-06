// Helper module for dealing with command line input
// This is essentailly a large clap call

const APP_AUTHOR: &str = "matt";
const APP_NAME: &str = "symbols";

pub const VERBOSITY_ARG: &str = "verbosity";
pub const CONFIG_FILE_ARG: &str = "config";

pub const UPLOAD_SUBCOMMAND: &str = "upload";
pub const UPLOAD_PATH_ARG: &str = "path";
pub const UPLOAD_RECUSRIVE_ARG: &str = "recursive";
pub const UPLOAD_DRY_RUN_ARG: &str = "dry-run";

pub fn parse_args<'a>() -> clap::ArgMatches<'a> {
    clap::App::new(APP_NAME)
        .version(env!("CARGO_PKG_VERSION"))
        .about("CLI tool for symbolserver.com")
        .author(APP_AUTHOR)
        .arg(
            clap::Arg::with_name(VERBOSITY_ARG)
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .arg(
            clap::Arg::with_name(CONFIG_FILE_ARG)
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Path to config file")
                .required(false)
                .takes_value(true),
        )
        .subcommand(
            clap::SubCommand::with_name(UPLOAD_SUBCOMMAND)
                .about("Upload the debug info files to a debug server")
                .arg(
                    clap::Arg::with_name(UPLOAD_PATH_ARG)
                        .help("Path to search for debug info files")
                        .required(true)
                        .index(1),
                )
                .arg(
                    clap::Arg::with_name(UPLOAD_RECUSRIVE_ARG)
                        .short("r")
                        .long("recursive")
                        .help("Search path recursively"),
                )
                .arg(
                    clap::Arg::with_name(UPLOAD_DRY_RUN_ARG)
                        .short("d")
                        .long("dry-run")
                        .help("Fake the upload part")
                        .long_help(
                            "Shows where the files would be uploaded, but does not run the upload",
                        ),
                ),
        )
        .get_matches()
}
