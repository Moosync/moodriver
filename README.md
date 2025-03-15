# Moosync WASM Extension Test CLI

This CLI tool allows testing of Moosync WASM extensions by:
1. Loading and initializing extensions
2. Sending commands to extensions
3. Handling UI requests from extensions

## Usage

```
Usage: extensions-wasm [OPTIONS] <WASM>

Arguments:
  <WASM>  Path to the wasm directory

Options:
  -t, --trace <TRACE>  Path to the trace file
  -d, --dir <DIR>      Path to the trace directory
  -h, --help           Print help
  -V, --version        Print version
```

```bash
moodriver -d ./traces/sample_trace.json ./ext.wasm
```

## Writing traces

TODO
