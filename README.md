# symbols

[![Build Status](https://mattschulte.visualstudio.com/dbg/_apis/build/status/schultetwin1.dbg?branchName=master)](https://mattschulte.visualstudio.com/dbg/_build/latest?definitionId=1&branchName=master)

symbols is a CLI tool to download and upload debug symbols for native code
(such as C,C++, or Rust) to symbol servers.

## Usage

## Compatability

## Configuration

The following values can be configured by the user

* cache-path
* urls

### cache-path

Sets the root directory to hold cached symbols and sources. This cache is
queried before servers are queried. Defaults to `$XDG_CACHE_PATH/symbols`.

### urls

The urls for the servers for which to download symbols or sources from. On
upload, the first url in the list of URLS will be used

`symbols` accepts configuration in the following order:

1) Command line arguments
2) Environmental variables
3) Config file

### Command line arguments

Please use `--help` to see the command line arguments

### Environmental variables

The following environmental variables can be used:

| Name                | Type   | Description                                                         |
|---------------------|--------|---------------------------------------------------------------------|
| SYMBOLS_CACHE_PATH  | string | The path to the cache for symbols and sources to be stored.         |
| SYMBOLS_VERSOSE     | int    | The verbosity of the symbols output. 0 = normal, 3 = trace logging. |
| SYMBOLS_TIMEOUT     | int    | Time
