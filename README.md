# Moodriver

This CLI tool allows testing of Moosync WASM extensions by:
1. Loading and initializing extensions
2. Sending commands to extensions
3. Handling UI requests from extensions

## Installation

```bash
cargo install --git https://github.com/Moosync/moodriver
```

## Usage

```
Usage: moodriver [OPTIONS] <WASM>

Arguments:
  <WASM>  Path to the wasm directory

Options:
  -t, --trace <TRACE>  Path to the trace file
  -d, --dir <DIR>      Path to the trace directory
  -v, --verbose...
  -h, --help           Print help
  -V, --version        Print version
```

```bash
moodriver -d ./traces/sample_trace.json ./ext.wasm
```

```bash
moodriver -v -d ./traces/sample_trace.json ./ext.wasm
```

```bash
moodriver -vv -d ./traces/sample_trace.json ./ext.wasm
```

## Writing traces

There are 2 components to a trace file:
1. **Commands**
2. **Requests**

### Commands
Commands are used to simulate user actions as they would happen in Moosync.

For example, you can send a command "seeked" to the extension to simulate an action of seeking a song. The below example of a command means that the song was seeked to position 0.
The expected property defines what is an appropriate response that should be received by the extension. In this case, the extension should respond with a null since "seeked" events expects no return value.
```json
{
  "type": "seeked",
  "data": [0],
  "expected": null
}
```

Lets consider another example of the command "getProviderScopes". We want our extension to respond with the scopes "scrobbles" and "accounts". Some commands like this require passing a package name to the command.
The below trace expects the extension to respond with the scopes "scrobbles" and "accounts" for the command.

```json
{
  "type": "getProviderScopes",
  "data": {
    "packageName": "moosync.lastfm"
  },
  "expected": ["scrobbles", "accounts"]
}
```

More commands can be be found at [moosync_edk::ExtensionExtraEvent](https://moosync.app/extensions-sdk/wasm-extension-rs/docs/wasm32-wasip1/doc/moosync_edk/enum.ExtensionExtraEvent.html) and [moosync_edk::ExtensionCommand](https://moosync.app/extensions-sdk/wasm-extension-rs/docs/wasm32-wasip1/doc/moosync_edk/enum.ExtensionCommand.html)

### Requests
The requests property can be used to simulate responses to requests sent by the extension. For eg, if the extension makes a call to "getSecure", we can reply back with a mock response.
The below trace replies back to a getSecure request with a key of "session"

```json
{
  "type": "getSecure",
  "data": {
    "key": "session",
    "value": "test"
  }
}
```

More requests can be found at [moosync_edk::MainCommandResponse](https://moosync.app/extensions-sdk/wasm-extension-rs/docs/wasm32-wasip1/doc/moosync_edk/enum.MainCommandResponse.html)

### Sample trace file
```json
{
  "$schema": "https://raw.githubusercontent.com/Moosync/moodriver/refs/heads/main/schema.json",
  "commands": [
    {
      "type": "seeked",
      "data": [0],
      "expected": null
    },
    {
      "type": "getProviderScopes",
      "data": {
        "packageName": "moosync.lastfm"
      },
      "expected": ["scrobbles", "accounts"]
    }
  ],
  "requests": [
    {
      "type": "getSecure",
      "data": {
        "key": "not_session",
        "value": "not_test"
      }
    },
    {
      "type": "getSecure",
      "data": {
        "key": "session",
        "value": "test"
      }
    }
  ]
}

```
