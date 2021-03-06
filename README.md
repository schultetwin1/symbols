# symbols

[![Build Status](https://mattschulte.visualstudio.com/dbg/_apis/build/status/schultetwin1.dbg?branchName=master)](https://mattschulte.visualstudio.com/dbg/_build/latest?definitionId=1&branchName=master)

symbols is a CLI tool to download and upload debug symbols for native code
(such as C,C++, or Rust) to symbol servers.

## Usage

## Compatability

## Configuration

`symbols` can be configured via command line arguments, environmental
variables, and a configuration file. The configuration is picked up in that
order.

### Command line arguments

Please use `--help` to see the command line arguments.

### Environmental variables

!!! Not yet implemented !!!

The following environmental variables can be used

| Name                | Type   | Description                                                         |
|---------------------|--------|---------------------------------------------------------------------|
| SYMBOLS_CACHE_PATH  | string | The path to the cache for symbols and sources to be stored.         |
| SYMBOLS_VERSOSE     | int    | The verbosity of the symbols output. 0 = normal, 3 = trace logging. |
| SYMBOLS_TIMEOUT     | int    | Timeout for each transaction with the server                        |

### Configuration file

```toml
# Example symbols config file

[[servers]]
access = "read"
type = "http"
url = "https://debuginfod.elfutils.org/"

[[servers]]
access = "readwrite"
type = "s3"
bucket = "matts-symbols"
region = "us-east-2"
profile = "default"
```

The configuration file is a TOML based file which can configure the local
cache path and the remote file servers.

#### Cache Path

The cache path can be configured using `cache = <path to cache>`.

```toml
cache = /tmp/symbols_cache
```

The cache defaults to `$XDG_CACHE_PATH/symbols`.

#### File Servers

File servers are configured in the servers array. Their are only two
supported types of servers currently (HTTP and S3). Each server must have the
following entries:

`type` - This should be either `s3` or `http`.

`access` - This should be either `read` or `readwrite`.

Each server also have fields that are specific to it's type. For HTTP the user must specify the URL.

```toml
[[servers]]
type = "http"
url = "https://debuginfod.elfutils.org/"
```

For S3 the user must specify the bucket name and the region. The user may
specfy a prefix for the key in the buckey and a profile name if the AWS
credentials are not default.

```toml
[[servers]]
access = "readwrite"
type = "s3"
bucket = "matts-symbols"
region = "us-east-2"
profile = "matt"
prefix = "symbols/"
```