use std::collections::HashMap;
use std::io::{self, Write};
use std::process;
use std::{fs, path::PathBuf};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Debug)]
struct Rules {
    categories: HashMap<String, Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct MoveRecord {
    from: PathBuf,
    to: PathBuf,
}

#[derive(Serialize, Deserialize, Debug)]
struct History {
    moves: Vec<MoveRecord>,
}

//Main commands
pub fn organize_files(
    path: Option<PathBuf>,
    dry_run: bool,
    output: &Option<PathBuf>,
    keep_original: bool,
    by: &String,
    recursive: bool,
) -> Result<()> {
    let path = match path {
        Some(path) => path,
        None => std::env::current_dir()
            .context("Could not determine the current working directory")?,
    };

    let rules = load_rules()?;

    let entries = fs::read_dir(&path)
        .with_context(|| format!("Could not read the directory '{}'", path.display()))?;

    let mut history = History { moves: Vec::new() };

    for entry in entries {
        // A single unreadable entry should not abort the whole run.
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                eprintln!(
                    "Warning: could not read an entry in '{}': {e}",
                    path.display()
                );
                continue;
            }
        };

        if entry.file_name().to_string_lossy().starts_with(".") {
            continue;
        }

        let entry_path = entry.path();


        if let Some(extension) = entry_path.extension() {
            let ext = extension.to_string_lossy().to_lowercase();
            

            for (category, extensions) in &rules.categories {
                if extensions.contains(&ext) {
                    let category_path = path.join(category);

                    let Some(file_name) = entry_path.file_name() else {
                        eprintln!(
                            "Warning: skipping '{}' (it has no file name)",
                            entry_path.display()
                        );
                        continue;
                    };

                    let destination = category_path.join(file_name);
                    let dest = destination.clone();

                    if !dry_run {
                        if let Err(e) = fs::create_dir_all(&category_path) {
                            eprintln!(
                                "Warning: could not create the folder '{}': {e}; skipping '{}'",
                                category_path.display(),
                                file_name.to_string_lossy()
                            );
                            continue;
                        }

                        let outcome = if keep_original {
                            fs::copy(&entry_path, &destination).map(|_| ())
                        } else {
                            fs::rename(&entry_path, &destination)
                        };

                        if let Err(e) = outcome {
                            let action = if keep_original { "copy" } else { "move" };
                            eprintln!(
                                "Warning: could not {action} '{}' to '{}': {e}; skipping",
                                entry_path.display(),
                                destination.display()
                            );
                            continue;
                        }
                    }

                    println!("{} moved to {}", file_name.to_string_lossy(), category);

                    history.moves.push(MoveRecord {
                        from: entry_path.clone(),
                        to: dest,
                    });

                    // A dry run must not touch the filesystem.
                    if !dry_run {
                        if let Some(out) = output {
                            if let Err(e) = fs::create_dir_all(out) {
                                eprintln!(
                                    "Warning: could not create the output folder '{}': {e}",
                                    out.display()
                                );
                                continue;
                            }
                            if let Err(e) = fs::rename(&category_path, out) {
                                eprintln!(
                                    "Warning: could not move '{}' to the output folder '{}': {e}",
                                    category_path.display(),
                                    out.display()
                                );
                                continue;
                            }
                        }
                    }
                }
            }
        } else if recursive {
            // Descend into sub-directories; report and skip ones we cannot handle.
            if let Err(e) = organize_files(
                Some(entry_path.clone()),
                dry_run,
                output,
                keep_original,
                by,
                recursive,
            ) {
                eprintln!("Warning: could not organize '{}': {e:#}", entry_path.display());
            }
        }
    }

    // History is only meaningful for real moves, so skip it on a dry run.
    if !dry_run {
        let json = serde_json::to_string_pretty(&history)
            .context("Could not serialize the move history")?;

        fs::create_dir_all(get_config_path()?)
            .context("Could not create the application data directory")?;

        let history_path = get_history_path()?;
        fs::write(&history_path, json).with_context(|| {
            format!("Could not write the history file '{}'", history_path.display())
        })?;
    }

    Ok(())
}

pub fn undo(history_path: &PathBuf) -> Result<()> {
    let json = fs::read_to_string(history_path).map_err(|e| match e.kind() {
        io::ErrorKind::NotFound => anyhow!(
            "Nothing to undo: no history file found at '{}'.",
            history_path.display()
        ),
        _ => anyhow::Error::new(e).context(format!(
            "Could not read the history file '{}'",
            history_path.display()
        )),
    })?;

    let history: History = serde_json::from_str(&json).with_context(|| {
        format!(
            "The history file '{}' contains invalid JSON",
            history_path.display()
        )
    })?;

    for movement in history.moves.iter().rev() {
        // Restore as much as possible: warn on a failure but keep going.
        if let Err(e) = fs::rename(&movement.to, &movement.from) {
            eprintln!(
                "Warning: could not restore '{}' to '{}': {e}",
                movement.to.display(),
                movement.from.display()
            );
        }
    }

    fs::remove_file(history_path).with_context(|| {
        format!(
            "Could not delete the history file '{}'",
            history_path.display()
        )
    })?;

    Ok(())
}

//rules management
pub fn save_rule(category: String, extensions: Vec<String>) -> Result<()> {
    let extension_text = extensions.join(", ");
    let category_text = category.clone();

    let rules_path = get_rules_path()?;
    if let Some(parent) = rules_path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("Could not create the config directory '{}'", parent.display())
        })?;
    }

    // Missing or empty file just means "no rules yet"; start from a default set.
    let json = fs::read_to_string(&rules_path).unwrap_or_default();
    let mut rules: Rules = serde_json::from_str(&json).unwrap_or_default();
    rules
        .categories
        .entry(category)
        .or_insert_with(Vec::new)
        .extend(extensions);

    let json = serde_json::to_string_pretty(&rules).context("Could not serialize the rules")?;
    fs::write(&rules_path, json)
        .with_context(|| format!("Could not write the rules file '{}'", rules_path.display()))?;

    println!("Rule {category_text} added with extensions {extension_text}");
    Ok(())
}

pub fn remove_rule(category: String, extensions: Option<Vec<String>>) -> Result<()> {
    let category_text = category.clone();

    let rules_path = get_rules_path()?;
    let json = fs::read_to_string(&rules_path).unwrap_or_default();
    let mut rules: Rules = serde_json::from_str(&json).unwrap_or_default();

    if let Some(exten) = extensions {
        let extension_text = exten.join(", ");
        if let Some(existing_extensions) = rules.categories.get_mut(&category) {
            existing_extensions.retain(|ext| !exten.contains(ext));
        }
        let json =
            serde_json::to_string_pretty(&rules).context("Could not serialize the rules")?;
        fs::write(&rules_path, json).with_context(|| {
            format!("Could not write the rules file '{}'", rules_path.display())
        })?;
        println!("Extensions {extension_text} removed from rule {category_text}");
    } else {
        confirm("Are you sure that you want to remove completely that category?")?;
        rules.categories.remove(&category);
        let json =
            serde_json::to_string_pretty(&rules).context("Could not serialize the rules")?;
        fs::write(&rules_path, json).with_context(|| {
            format!("Could not write the rules file '{}'", rules_path.display())
        })?;
        println!("Rule {category_text} removed");
    }

    Ok(())
}

fn load_rules() -> Result<Rules> {
    let rules_path = get_rules_path()?;

    let json = fs::read_to_string(&rules_path).map_err(|e| match e.kind() {
        io::ErrorKind::NotFound => anyhow!(
            "No rules file found at '{}'. Add a category first, e.g. `organizer add-rule images -e jpg png`.",
            rules_path.display()
        ),
        _ => anyhow::Error::new(e).context(format!(
            "Could not read the rules file '{}'",
            rules_path.display()
        )),
    })?;

    serde_json::from_str(&json).with_context(|| {
        format!(
            "The rules file '{}' contains invalid JSON",
            rules_path.display()
        )
    })
}

pub fn show_rules() -> Result<()> {
    let rules = load_rules()?;
    if rules.categories.is_empty() {
        println!("No rules added yet");
        return Ok(());
    }
    for (category, extensions) in rules.categories {
        let extensions_string = extensions.join(", ");
        println!("{category}: {extensions_string}");
    }
    Ok(())
}

//config
pub fn get_rules_path() -> Result<PathBuf> {
    let mut path =
        dirs::config_dir().context("Could not determine the system configuration directory")?;
    path.push("organizer");
    path.push("rules.json");
    Ok(path)
}

pub fn get_history_path() -> Result<PathBuf> {
    let mut path =
        dirs::data_local_dir().context("Could not determine the local data directory")?;
    path.push("organizer");
    path.push("history.json");
    Ok(path)
}

pub fn get_config_path() -> Result<PathBuf> {
    let mut path =
        dirs::data_local_dir().context("Could not determine the local data directory")?;
    path.push("organizer");
    Ok(path)
}

//utils
pub fn confirm(mensaje: &str) -> Result<()> {
    print!("{mensaje} [escribe 'yes' para continuar]: ");
    io::stdout()
        .flush()
        .context("Could not write to standard output")?;

    let mut entrada = String::new();
    io::stdin()
        .read_line(&mut entrada)
        .context("Could not read from standard input")?;

    if entrada.trim() != "yes" {
        println!("Acción cancelada.");
        process::exit(0);
    }

    Ok(())
}
