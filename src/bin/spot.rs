use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const SKILL_FILES: &[(&str, &str)] = &[
    (
        "SKILL.md",
        include_str!("../../skills/spot-game-builder/SKILL.md"),
    ),
    (
        "agents/openai.yaml",
        include_str!("../../skills/spot-game-builder/agents/openai.yaml"),
    ),
    (
        "references/repo-map.md",
        include_str!("../../skills/spot-game-builder/references/repo-map.md"),
    ),
];

fn print_help() {
    println!(
        "Spot command-line tools\n\n\
         Usage:\n  \
           spot install-skill [--project <directory>]\n  \
           spot --help\n\n\
         Commands:\n  \
           install-skill  Install the Spot game-builder skill in a project\n\n\
         Options:\n  \
           --project <directory>  Target project (defaults to the current directory)"
    );
}

fn parse_install_skill_args(args: &[String]) -> Result<PathBuf, String> {
    let mut project = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--project" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--project requires a directory".to_string())?;
                if project.replace(PathBuf::from(value)).is_some() {
                    return Err("--project may only be supplied once".to_string());
                }
            }
            "-h" | "--help" => {
                print_help();
                return Err(String::new());
            }
            argument => return Err(format!("unknown argument: {argument}")),
        }
        index += 1;
    }

    match project {
        Some(path) => Ok(path),
        None => {
            env::current_dir().map_err(|error| format!("cannot read current directory: {error}"))
        }
    }
}

fn install_skill(project: &Path) -> Result<PathBuf, String> {
    if !project.is_dir() {
        return Err(format!(
            "project directory does not exist: {}",
            project.display()
        ));
    }

    let destination = project.join(".agents/skills/spot-game-builder");
    for (relative_path, contents) in SKILL_FILES {
        let output_path = destination.join(relative_path);
        let parent = output_path
            .parent()
            .ok_or_else(|| format!("invalid skill path: {}", output_path.display()))?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        fs::write(&output_path, contents)
            .map_err(|error| format!("cannot write {}: {error}", output_path.display()))?;
    }

    Ok(destination)
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("install-skill") => {
            let project = parse_install_skill_args(&args[1..])?;
            let destination = install_skill(&project)?;
            println!("Installed spot-game-builder to {}", destination.display());
            Ok(())
        }
        Some("-h" | "--help") | None => {
            print_help();
            Ok(())
        }
        Some(command) => Err(format!("unknown command: {command}")),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.is_empty() => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}\n\nRun `spot --help` for usage.");
            ExitCode::FAILURE
        }
    }
}
