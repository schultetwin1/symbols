// Helper module for dealing with command line input
// This is essentailly a large clap call

use clap::ArgAction;

const APP_AUTHOR: &str = "matt";
const APP_NAME: &str = "symbols";

pub const VERBOSITY_ARG: &str = "verbosity";
pub const CONFIG_FILE_ARG: &str = "config";

pub const UPLOAD_SUBCOMMAND: &str = "upload";
pub const UPLOAD_PATH_ARG: &str = "path";
pub const UPLOAD_RECUSRIVE_ARG: &str = "recursive";
pub const UPLOAD_DRY_RUN_ARG: &str = "dry-run";
pub const UPLOAD_SERVER_NAME_ARG: &str = "server";
pub const UPLOAD_OUTPUT_DIR_ARG: &str = "output";
pub const UPLOAD_S3_BUCKET_ARG: &str = "s3bucket";
pub const UPLOAD_S3_REGION_ARG: &str = "s3region";

pub const LOGIN_SUBCOMMAND: &str = "login";
pub const LOGIN_SERVICE_ARG: &str = "service";

pub fn parse_args() -> clap::ArgMatches {
    clap::Command::new(APP_NAME)
        .version(env!("CARGO_PKG_VERSION"))
        .about("CLI tool for symbolserver.com")
        .author(APP_AUTHOR)
        .arg(
            clap::Arg::new(VERBOSITY_ARG)
                .short('v')
                .action(ArgAction::Count)
                .help("Sets the level of verbosity"),
        )
        .arg(
            clap::Arg::new(CONFIG_FILE_ARG)
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Path to config file")
                .required(false)
                .action(ArgAction::Set)
        )
        .subcommand(
          clap::Command::new(UPLOAD_SUBCOMMAND)
                .about("Upload the debug info files to a debug server")
                .arg(
                    clap::Arg::new(UPLOAD_PATH_ARG)
                        .help("Path to search for debug info files")
                        .required(true)
                        .index(1),
                )
                .arg(
                    clap::Arg::new(UPLOAD_RECUSRIVE_ARG)
                        .short('r')
                        .long("recursive")
                        .action(ArgAction::SetTrue)
                        .help("Search path recursively"),
                )
                .arg(
                    clap::Arg::new(UPLOAD_DRY_RUN_ARG)
                        .short('d')
                        .long("dry-run")
                        .action(ArgAction::SetTrue)
                        .help("Fake the upload part")
                        .long_help(
                            "Shows where the files would be uploaded, but does not run the upload",
                        ),
                )
                .arg(
                    clap::Arg::new(UPLOAD_SERVER_NAME_ARG)
                        .short('s')
                        .long("server")
                        .help("Name of server in config file")
                        .long_help("Specify which server in config file to upload files too")
                        .required(false)
                        .action(ArgAction::Set)
                )
                .arg(
                    clap::Arg::new(UPLOAD_OUTPUT_DIR_ARG)
                        .short('o')
                        .long("output-dir")
                        .help("Output directory for symbols")
                        .long_help("Copy all symbols to the given folder (and do not upload to any web service")
                        .required(false)
                        .action(ArgAction::Set)
                        .conflicts_with(UPLOAD_S3_BUCKET_ARG)
                )
                .arg(
                    clap::Arg::new(UPLOAD_S3_BUCKET_ARG)
                        .long("s3-bucket")
                        .help("S3 bucket to upload symbols too")
                        .conflicts_with(UPLOAD_OUTPUT_DIR_ARG)
                        .requires(UPLOAD_S3_REGION_ARG)
                        .required(false)
                        .action(ArgAction::Set)
                )
                .arg(
                    clap::Arg::new(UPLOAD_S3_REGION_ARG)
                        .long("s3-region")
                        .help("S3 region to upload symbols too")
                        .conflicts_with(UPLOAD_OUTPUT_DIR_ARG)
                        .requires(UPLOAD_S3_BUCKET_ARG)
                        .required(false)
                        .action(ArgAction::Set)
                )

        )
        .subcommand(
            clap::Command::new(LOGIN_SUBCOMMAND)
            .about("Login to web services in order to download sources / symbols")
            .arg(
                clap::Arg::new(LOGIN_SERVICE_ARG)
                    .help("The service to log in to")
                    .value_parser(["github", "symbolserver"])
                    .default_value("symbolserver")
                    .action(ArgAction::Set)
                    .required(false)
                    .index(1),
                )
        )
        .get_matches()
}
