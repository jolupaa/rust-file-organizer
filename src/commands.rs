use std::{fs, path::{PathBuf}};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
pub fn organize_files(path: Option<PathBuf>, dry_run: bool, output: &Option<PathBuf>, keep_original: bool, by: &String, recursive:bool   ) {
    let path = path.unwrap_or_else(|| std::env::current_dir().unwrap());
    let rules = load_rules();
    
    let entries = std::fs::read_dir(&path).unwrap();

    let mut history = History { moves: Vec::new() };

    for entry in entries{
        let entry = entry.unwrap();
        let entry_path = entry.path();

        if let Some(extension) = entry_path.extension()
        {
            let ext = extension.to_string_lossy().to_lowercase();

            for (category, extensions) in &rules.categories
            {
                if extensions.contains(&ext) {
                    let category_path = path.join(category);
                    let file_name = entry_path.file_name().unwrap();
                    let destination = category_path.join(file_name);
                    let dest = destination.clone();

                    if !dry_run {
                        fs::create_dir_all(&category_path).unwrap();
                        if keep_original {
                            fs::copy(&entry_path, destination).unwrap();
                        }
                        else{
                            fs::rename(&entry_path, destination).unwrap();
                        }
                        
                    }

                    println!("{} moved to {}", file_name.to_string_lossy(), category);
                    let record = MoveRecord {
                        from: entry_path.clone(),
                        to: dest
                    };

                    history.moves.push(record);

                    if let Some(out) = output {
                        fs::create_dir_all(&out).unwrap();
                        fs::rename(&category_path, &out).unwrap();
                    }
                   
                }
            }
        }
        else if recursive {
            organize_files(Some(entry_path), dry_run, output, keep_original, by, recursive);
        }

    }

    let json = serde_json::to_string_pretty(&history).unwrap();
    std::fs::write(get_history_path(), json).unwrap();

}

pub fn undo(history_path: &PathBuf) {
    let json = fs::read_to_string(history_path).unwrap();
    let history: History = serde_json::from_str(&json).unwrap();

    for movement in history.moves.iter().rev() {
        let from = PathBuf::from(&movement.to);
        let to = PathBuf::from(&movement.from);

        fs::rename(from, to).unwrap();
    }

    fs::remove_file(history_path).unwrap();
}


//rules management
pub fn save_rule(category: String, extensions: Vec<String>) {
    let extension_text = extensions.join(", ");
    let category_text = category.clone();
    fs::create_dir_all(get_config_path()).unwrap();


    let json = fs::read_to_string(get_rules_path()).unwrap_or_default();
    let mut rules: Rules = serde_json::from_str(&json).unwrap_or_default();
    rules.categories.entry(category).or_insert_with(Vec::new).extend(extensions);
    let r: &Rules = &rules;
    let json = serde_json::to_string_pretty(r).unwrap();
    let added = std::fs::write(get_rules_path(), json);
    match added {
        Ok(()) => {println!("Rule {} added with extensions {}", category_text, extension_text)}
        _err => {println!("Error updating rules.json")}     
    }
}

pub fn remove_rule(category: String, extensions: Option<Vec<String>>) {
    let category_text = category.clone();

    let json = fs::read_to_string(get_rules_path()).unwrap_or_default();
    let mut rules: Rules = serde_json::from_str(&json).unwrap_or_default();
    if let Some(exten) = extensions {
        let extension_text = exten.join(", ");
        if let Some(existing_extensions) = rules.categories.get_mut(&category) {
            existing_extensions.retain(|ext| !exten.contains(ext));
            
        }
        let json = serde_json::to_string_pretty(&rules).unwrap();
        fs::write(get_rules_path(), json).unwrap();
        println!("Extensions {} removd from rule {}", extension_text, category_text)
    } else {
        confirm("Are you sure that you want to remove completely that category?");
        rules.categories.remove(&category);
        let r: &Rules = &rules;
        let json = serde_json::to_string_pretty(r).unwrap();
        std::fs::write(get_rules_path(), json).unwrap();
        println!("Rule {} removed", category_text)
    }
}

fn load_rules() -> Rules {
    let json = fs::read_to_string(get_rules_path()).unwrap();
    let rules: Rules = serde_json::from_str(&json).unwrap();
    rules
}

pub fn show_rules() {
    let rules = load_rules();
    if rules.categories.is_empty() {
        println!("No rules added yet");
        return
    }
    for (category, extensions) in rules.categories {
        let extensions_string = extensions.join(", ");
        println!("{}: {}", category, extensions_string);
    }
}

//conifg
pub fn get_rules_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap();
    path.push("organizer");
    path.push("rules.json");
    path
}

pub fn get_history_path() -> PathBuf {
    let mut path = dirs::data_local_dir().unwrap();
    path.push("organizer");
    path.push("history.json");
    path
}
pub fn get_config_path() -> PathBuf {
    let mut path = dirs::data_local_dir().unwrap();
    path.push("organizer");
    path
}

//utils
use std::io::{self, Write};
use std::process;

pub fn confirm(mensaje: &str) {
    print!("{} [escribe 'yes' para continuar]: ", mensaje);
    io::stdout().flush().expect("No se pudo escribir en stdout");

    let mut entrada = String::new();
    io::stdin()
        .read_line(&mut entrada)
        .expect("No se pudo leer la entrada");

    if entrada.trim() != "yes" {
        println!("Acción cancelada.");
        process::exit(0);
    }
}