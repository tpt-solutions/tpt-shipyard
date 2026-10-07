//! `tpt-yard` — the command-line front end for tpt-shipyard.
//!
//! Subcommands (the table lives in `help.rs`, the flag specs in `args.rs`):
//!
//! - `validate FILE.json` — load a vessel project (or hull manifest) and
//!   report every inconsistency; exit 1 on failure.
//! - `plan MANIFEST.json` — the end-to-end construction plan for a hull
//!   manifest: block division → erection order → lift checks → schedule →
//!   critical path, one report.
//! - `export MANIFEST.json [--gltf out.gltf] [--ifc out.ifc]` — block
//!   division written out as glTF 2.0 (merged mesh) and/or IFC4 STEP
//!   (one named product per block) for web viewers and BIM tools.
//! - `stability CASE.json [--offsets F.csv] [--draft M] [--kg M] [--csv PREFIX]`
//!   `[--svg F.svg] [--strict]` — hydrostatics, GZ curve and the IMO 2008
//!   criteria from a hull-offsets table or a prismatic screen (`stability.rs`).
//! - `new-hull <wigley|barge|workboat|tug|sailboat|ferry> [--out DIR]` — write a
//!   runnable hull (offsets CSV + stability case) to start from (`hulls.rs`).
//! - `import-hull MESH.obj|.stl [--up y|z] [--bow +x|-x] [--scale F]` — slice a
//!   hull surface mesh into an offsets CSV + stability case (`meshhull.rs`).
//! - `schedule FILE.json [--limit kind=value]...` — CPM critical path and
//!   resource levelling for a vessel project's activities (`sched.rs`).
//! - `risk FILE.json ...` — Monte Carlo schedule risk (`sched.rs`).
//! - `report FILE.json` — weight/CoG/structural summary for a project.
//! - `new <sea|space> [NAME]` — print a scaffolded project JSON built from
//!   the workspace templates (`project.rs`).
//! - `html-report` / `pdf-report` — the calculation package (`reports.rs`).
//!
//! Every command takes `--json` for machine-readable output. Exit codes:
//! `0` ok, `1` a check failed (`validate`, `--strict`), `2` usage error,
//! `3` runtime or I/O failure.

mod args;
mod export_cmd;
mod help;
mod hulls;
mod meshhull;
mod project;
mod reports;
mod sched;
mod stability;

use std::process::ExitCode;

use args::CliError;

fn main() -> ExitCode {
    install_pipe_hook();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (json_mode, args) = split_flag(&args, "--json");
    match run(&args, json_mode) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            // Usage text only for command-line mistakes, not for runtime
            // failures such as a missing file.
            if matches!(e, CliError::Usage(_)) {
                match args.first().and_then(|c| help::usage_of(c)) {
                    Some(usage) => eprintln!("\n{usage}"),
                    None => eprintln!("run `tpt-yard --help` for the command list"),
                }
            }
            ExitCode::from(e.exit_code())
        }
    }
}

/// `println!` panics when stdout is a closed pipe (`tpt-yard ... | head`);
/// treat that as a normal early exit instead of a crash.
fn install_pipe_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let payload = info.payload();
        let msg = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&str>().copied())
            .unwrap_or("");
        if msg.contains("failed printing to stdout") || msg.contains("Broken pipe") {
            std::process::exit(0);
        }
        default(info);
    }));
}

fn split_flag<'a>(args: &'a [String], flag: &str) -> (bool, Vec<&'a str>) {
    let mut json = false;
    let mut rest = Vec::new();
    for a in args {
        if a == flag {
            json = true;
        } else {
            rest.push(a.as_str());
        }
    }
    (json, rest)
}

fn run(args: &[&str], json_mode: bool) -> Result<(), CliError> {
    let Some((cmd, rest)) = args.split_first() else {
        print!("{}", help::overview());
        return Err(CliError::Usage("missing subcommand".into()));
    };
    match *cmd {
        "--help" | "-h" => {
            print!("{}", help::overview());
            return Ok(());
        }
        "--version" | "-V" => {
            println!("tpt-yard {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        "help" => {
            return match rest {
                [] => {
                    print!("{}", help::overview());
                    Ok(())
                }
                [c] => {
                    let usage =
                        help::usage_of(c).ok_or_else(|| CliError::Usage(unknown_command(c)))?;
                    print!("{usage}");
                    Ok(())
                }
                _ => Err(CliError::Usage("help takes one command name".into())),
            };
        }
        _ => {}
    }
    let spec = args::spec_for(cmd).ok_or_else(|| CliError::Usage(unknown_command(cmd)))?;
    let parsed = args::parse(rest, spec)?;
    if parsed.help {
        print!(
            "{}",
            help::usage_of(cmd).ok_or_else(|| CliError::Usage(unknown_command(cmd)))?
        );
        return Ok(());
    }
    match *cmd {
        "validate" => project::validate(&parsed, json_mode),
        "plan" => project::plan(&parsed, json_mode),
        "report" => project::report(&parsed, json_mode),
        "new" => project::new(&parsed, json_mode),
        "export" => export_cmd::export(&parsed, json_mode),
        "schedule" => sched::schedule(&parsed, json_mode),
        "risk" => sched::risk(&parsed, json_mode),
        "html-report" => reports::html_report(&parsed, json_mode),
        "pdf-report" => reports::pdf_report(&parsed, json_mode),
        // These three keep their own (already position-independent) parsers;
        // the spec above has validated the command line first.
        "stability" => stability::run(rest, json_mode).map_err(CliError::from),
        "new-hull" => hulls::run(rest, json_mode).map_err(CliError::from),
        "import-hull" => meshhull::run(rest, json_mode).map_err(CliError::from),
        other => Err(CliError::Usage(unknown_command(other))),
    }
}

fn unknown_command(cmd: &str) -> String {
    match help::suggest(cmd) {
        Some(near) => format!("unknown subcommand '{cmd}' — did you mean '{near}'?"),
        None => format!("unknown subcommand '{cmd}'"),
    }
}
