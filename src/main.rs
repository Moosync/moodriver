use std::{
    collections::HashMap,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    str::FromStr,
    thread,
    time::Duration,
};

use clap::{ArgAction, Parser, arg, command};
use colored::*;
use extensions::{ExtensionHandler, UiReplySender, UiRequestReceiver};
use json_comments::StripComments;
use serde::Deserialize;
use serde_json::Value;
use tracing::{create_log_buffer, create_verbose_log, flush_logs};
use types::{
    errors::{MoosyncError, Result},
    extensions::{ExtensionCommand, GenericExtensionHostRequest, MainCommand, MainCommandResponse},
    songs::Song,
    ui::{
        extensions::{ExtensionExtraEvent, ExtensionExtraEventArgs, PreferenceData},
        player_details::PlayerState,
    },
};
use ui::finish_and_clear;
use walkdir::WalkDir;

mod tracing;
mod ui;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the trace file
    #[arg(short = 't', long = "trace", conflicts_with = "dir")]
    trace: Option<PathBuf>,

    /// Path to the trace directory
    #[arg(short = 'd', long = "dir", conflicts_with = "trace")]
    dir: Option<PathBuf>,

    /// Path to the wasm directory
    wasm: PathBuf,

    #[arg(short = 'v', long = "verbose", default_value = "0", action = ArgAction::Count)]
    verbose: u8,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub(crate) enum ValidCommand {
    ExtensionExtraEvent(ExtensionExtraEvent),
    ExtensionCommand(ExtensionCommand),
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct CommandWrapper {
    #[serde(flatten)]
    command: ValidCommand,
    expected: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type", content = "data")]
pub(crate) enum MainCommandParsable {
    GetSong(Vec<Song>),
    GetEntity(Value),
    GetCurrentSong(Option<Song>),
    GetPlayerState(PlayerState),
    GetVolume(f64),
    GetTime(f64),
    GetQueue(Value),
    GetPreference(PreferenceData),
    SetPreference(bool),
    GetSecure(PreferenceData),
    SetSecure(bool),
    AddSongs(Vec<Song>),
    RemoveSong(bool),
    UpdateSong(Song),
    AddPlaylist(String),
    AddToPlaylist(bool),
    RegisterOAuth(bool),
    OpenExternalUrl(bool),
    UpdateAccounts(bool),
    ExtensionsUpdated(bool),
}

#[derive(Debug, Deserialize)]
struct TestCase {
    commands: Vec<CommandWrapper>,
    requests: Vec<MainCommandParsable>,
}

fn setup_ext_handler(
    ext_dir: PathBuf,
) -> Result<(ExtensionHandler, UiRequestReceiver, UiReplySender)> {
    let (handler, ui_req_rx, ui_reply_tx) = ExtensionHandler::new(
        ext_dir,
        PathBuf::from_str("/tmp/ext-tmp").unwrap(),
        PathBuf::from_str("/tmp/ext-tmp-cache").unwrap(),
    );

    Ok((handler, ui_req_rx, ui_reply_tx))
}

fn parse_test_case(test_file: &Path) -> Result<TestCase> {
    let file = File::open(test_file)?;
    let reader = BufReader::new(file);
    let stripped = StripComments::new(reader);

    let ext = test_file
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .ok_or_else(|| MoosyncError::String("Missing or invalid file extension".into()))?;

    let test_case: TestCase = match ext.as_str() {
        "json" | "jsonc" => serde_json::from_reader(stripped)?,
        "yaml" | "yml" => {
            serde_yaml::from_reader(stripped).map_err(|e| MoosyncError::String(e.to_string()))?
        }
        _ => return Err("Unsupported file extension".into()),
    };

    Ok(test_case)
}

fn find_matching_preference<'a>(
    pref_data: &PreferenceData,
    requests: &'a [MainCommandParsable],
    variant_name: &str,
) -> Option<&'a MainCommandParsable> {
    let matching_requests: Vec<&MainCommandParsable> = match variant_name {
        "GetPreference" => requests
            .iter()
            .filter(|r| matches!(r, MainCommandParsable::GetPreference(_)))
            .collect(),
        "GetSecure" => requests
            .iter()
            .filter(|r| matches!(r, MainCommandParsable::GetSecure(_)))
            .collect(),
        _ => Vec::new(),
    };

    matching_requests
        .into_iter()
        .find(|r| match (r, variant_name) {
            (MainCommandParsable::GetPreference(request_data), "GetPreference") => {
                request_data.key == pref_data.key
            }
            (MainCommandParsable::GetSecure(request_data), "GetSecure") => {
                request_data.key == pref_data.key
            }
            _ => false,
        })
}

macro_rules! define_command_mappings {
    (
        with_params: [$($with_params:ident),* $(,)?],
        no_params: [$($no_params:ident),* $(,)?],
        preference_commands: [$($pref_command:ident),* $(,)?]
    ) => {
        fn find_matching_request<'a>(command: &MainCommand, requests: &'a [MainCommandParsable]) -> Option<&'a MainCommandParsable> {
            match command {
                $(
                    MainCommand::$pref_command(pref_data) => {
                        find_matching_preference(pref_data, requests, stringify!($pref_command))
                    },
                )*

                $(
                    MainCommand::$with_params(_) => requests.iter().find(|r| matches!(r, MainCommandParsable::$with_params(_))),
                )*

                $(
                    MainCommand::$no_params() => requests.iter().find(|r| matches!(r, MainCommandParsable::$no_params(_))),
                )*
            }
        }

        fn create_response_from_request(request: &MainCommandParsable) -> MainCommandResponse {
            match request {
                $(
                    MainCommandParsable::$with_params(data) => MainCommandResponse::$with_params(data.clone()),
                )*
                $(
                    MainCommandParsable::$no_params(data) => MainCommandResponse::$no_params(data.clone()),
                )*
                $(
                    MainCommandParsable::$pref_command(data) => MainCommandResponse::$pref_command(data.clone()),
                )*
            }
        }

        fn create_default_response(command: &MainCommand) -> MainCommandResponse {
            match command {
                $(
                    MainCommand::$with_params(_) => MainCommandResponse::$with_params(Default::default()),
                )*
                $(
                    MainCommand::$no_params() => MainCommandResponse::$no_params(Default::default()),
                )*
                $(
                    MainCommand::$pref_command(_) => MainCommandResponse::$pref_command(Default::default()),
                )*
            }
        }

        fn create_response(command: &MainCommand, requests: &[MainCommandParsable]) -> MainCommandResponse {
            if let Some(request) = find_matching_request(command, requests) {
                create_response_from_request(request)
            } else {
                create_default_response(command)
            }
        }
    };
}

define_command_mappings!(
    with_params: [
        GetSong, GetEntity, SetPreference, SetSecure,
        AddSongs, RemoveSong, UpdateSong, AddPlaylist, AddToPlaylist, RegisterOAuth,
        OpenExternalUrl, UpdateAccounts
    ],

    no_params: [
        GetCurrentSong, GetPlayerState, GetVolume, GetTime, GetQueue, ExtensionsUpdated
    ],

    preference_commands: [
        GetPreference, GetSecure
    ]
);

fn listen_ui_requests(
    mut ui_requests_rx: UiRequestReceiver,
    ui_reply_tx: UiReplySender,
    requests: Vec<MainCommandParsable>,
) {
    tokio::spawn(async move {
        loop {
            if let Some(request) = ui_requests_rx.recv().await {
                if let Some(command) = request.data {
                    let request_description = match &command {
                        MainCommand::GetPreference(pref) => {
                            format!("GetPreference with key '{}'", pref.key)
                        }
                        MainCommand::GetSecure(pref) => {
                            format!("GetSecure with key '{}'", pref.key)
                        }
                        other => format!("{:?}", other),
                    };

                    let response = create_response(&command, &requests);

                    let response_value = match &response {
                        MainCommandResponse::GetPreference(data) => {
                            format!("data for key '{}': '{:?}'", data.key, data.value)
                        }
                        MainCommandResponse::GetSecure(data) => {
                            format!("data for key '{}': '{:?}'", data.key, data.value)
                        }
                        other => format!("{:?}", other),
                    };

                    ui::log_ui_request(&request_description, &response_value).await;

                    ui_reply_tx
                        .send(GenericExtensionHostRequest {
                            channel: request.channel,
                            data: Some(response),
                        })
                        .unwrap();
                }
            }
        }
    });
}

fn is_ignore(expected: &Value) -> bool {
    expected.is_string() && expected.as_str().unwrap() == "ignore"
}

async fn run_test(file: &Path, wasm: &Path) -> Result<()> {
    let test_case = parse_test_case(file)?;
    println!(
        "{} {} commands and {} requests\n",
        "Loaded test case with".blue(),
        test_case.commands.len(),
        test_case.requests.len()
    );

    let (handler, ui_requests_rx, ui_reply_tx) =
        setup_ext_handler(wasm.parent().unwrap().to_path_buf())?;

    listen_ui_requests(ui_requests_rx, ui_reply_tx, test_case.requests);

    handler.find_new_extensions().await?;

    let mut is_waiting: bool = true;

    ui::initialize_progress_bar().await;

    let mut notified: HashMap<String, bool> = HashMap::new();
    while is_waiting {
        is_waiting = true;
        let exts = handler.get_installed_extensions().await?;
        let mut active = 0;
        for ext in exts.iter() {
            if !notified.contains_key(&ext.package_name) {
                notified.insert(ext.package_name.clone(), true);
                println!(
                    "Extension found {}, active: {}",
                    ext.package_name, ext.active
                );
            }
            if ext.active {
                active += 1;
            }
        }

        if !exts.is_empty() && active == exts.len() {
            is_waiting = false
        } else {
            thread::sleep(Duration::from_millis(1000));
        }
    }

    if !is_waiting {
        finish_and_clear().await;
    }

    let package_name = handler
        .get_installed_extensions()
        .await?
        .first()
        .unwrap()
        .package_name
        .clone();

    println!("Extension active: {}", package_name.yellow());

    println!("\n------------------------------------------------------------");
    println!(
        "{} {} {}",
        "=== Running commands from test case".cyan(),
        file.to_string_lossy().cyan(),
        "... ===".cyan()
    );

    let total_commands = test_case.commands.len();
    for (i, command) in test_case.commands.into_iter().enumerate() {
        let command_desc = match &command.command {
            ValidCommand::ExtensionExtraEvent(event) => {
                format!("ExtensionExtraEvent[type: {:?}]", event)
            }
            ValidCommand::ExtensionCommand(cmd) => format!("ExtensionCommand[{:?}]", cmd),
        };

        println!(
            "\nCommand [{}/{}]: {}",
            i + 1,
            total_commands,
            command_desc.magenta()
        );

        let resp = match command.command {
            ValidCommand::ExtensionExtraEvent(command) => {
                handler
                    .send_extension_command(
                        ExtensionCommand::ExtraExtensionEvent(ExtensionExtraEventArgs {
                            data: command.clone(),
                            package_name: package_name.clone(),
                        }),
                        true,
                    )
                    .await?
            }
            ValidCommand::ExtensionCommand(command) => {
                handler
                    .send_extension_command(command.clone(), true)
                    .await?
            }
        };

        if let Some(expected) = command.expected {
            if !is_ignore(&expected) {
                let resp_value = serde_json::to_value(resp)?;
                if resp_value != expected {
                    return Err(
                        format!("Expected: {:?}, received: {:?}", expected, resp_value).into(),
                    );
                }
            } else {
                if serde_json::to_value(&resp).unwrap().is_null() {
                    return Err("Expected: non null, received: null".into());
                }
            }
        } else if !serde_json::to_value(&resp).unwrap().is_null() {
            return Err(format!("Expected: null, received: {:?}", resp).into());
        }

        println!("✓ Successful: {}", command_desc.green());
    }

    println!(
        "{} {} {}",
        "=== Completed test case".cyan(),
        file.to_string_lossy().cyan(),
        "... ===".cyan()
    );

    Ok(())
}

async fn run_cli(mut args: Cli) -> Result<()> {
    println!(
        "{}",
        "=== Starting test CLI for WASM extensions ===\n".green()
    );

    if args.dir.is_none() {
        args.dir = Some(PathBuf::from_str("./traces").unwrap())
    }

    if let Some(trace) = args.trace {
        run_test(&trace, &args.wasm).await?;
    } else if let Some(dir) = args.dir {
        assert!(dir.exists(), "Traces directory {:?} does not exist", dir);

        for entry in WalkDir::new(&dir).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "json" || ext == "jsonc" || ext == "yaml" || ext == "yml" {
                        run_test(entry.path(), &args.wasm).await?;
                    }
                }
            }
        }
    }

    println!(
        "\n{}\n",
        "=== All test commands completed successfully ===".green()
    );

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    if args.verbose > 0 {
        create_verbose_log(args.verbose);
    } else {
        create_log_buffer();
    }

    if let Err(e) = run_cli(args.clone()).await {
        println!("{}", e.to_string().red());
        println!("\n=== Extension output ===\n",);
        flush_logs();
        println!("\n=== End Extension output ===\n",);
    }

    Ok(())
}
