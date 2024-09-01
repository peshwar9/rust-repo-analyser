use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;
use regex::Regex;
use walkdir::WalkDir;
use std::collections::HashMap;
use std::collections::HashSet;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <repository_url> <workspace_name>", args[0]);
        return;
    }

    let repo_url = &args[1];
    let workspace_name = &args[2];
    let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory");
    
    // Clone the repository
    clone_repo(repo_url, temp_dir.path());

    // Analyze the repository
    let workspace_path = temp_dir.path().join(workspace_name).join("src");
    let (file_count, total_non_blank_non_comment_lines) = count_files_and_lines(&workspace_path);
    let module_structure = analyze_modules(&workspace_path);

    // Print high-level results
    println!("Total Rust source files: {}", file_count);
    println!("Total non-blank and non-commented lines: {}", total_non_blank_non_comment_lines);
    println!("\nModules:");

    let modules: Vec<String> = module_structure.keys().cloned().collect();
    for (i, module) in modules.iter().enumerate() {
        println!("  {}. {} ({} lines)", i + 1, module, total_lines_in_module(&module_structure, module));
    }

    // Interactive CLI
    loop {
        print!("Enter the number of the module to view details, or 'q' to quit: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        if input == "q" {
            break;
        }

        if let Ok(selection) = input.parse::<usize>() {
            if selection > 0 && selection <= modules.len() {
                let selected_module = &modules[selection - 1];
                println!("\nModule: {}", selected_module);
                if let Some(files) = module_structure.get(selected_module) {
                    for (file, (line_count, functions, data_structures)) in files {
                        println!("  File: {} ({} lines)", file, line_count);
                        println!("    Functions:");
                        for function in functions {
                            println!("      {}", function);
                        }
                        println!("    Data structures:");
                        for (structure, count) in data_structures {
                            println!("      {}: {} times", structure, count);
                        }
                    }
                }
            } else {
                println!("Invalid selection. Please try again.");
            }
        } else {
            println!("Invalid input. Please enter a number or 'q' to quit.");
        }
    }
}

fn clone_repo(url: &str, path: &Path) {
    Command::new("git")
        .args(&["clone", url, path.to_str().unwrap()])
        .output()
        .expect("Failed to clone repository");
}

fn count_files_and_lines(path: &Path) -> (usize, usize) {
    let mut file_count = 0;
    let mut total_non_blank_non_comment_lines = 0;

    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        if entry.path().extension().map_or(false, |ext| ext == "rs") {
            file_count += 1;
            let content = fs::read_to_string(entry.path()).expect("Failed to read file");
            let non_blank_non_comment_count = count_non_blank_non_comment_lines(&content);
            total_non_blank_non_comment_lines += non_blank_non_comment_count;
        }
    }

    (file_count, total_non_blank_non_comment_lines)
}

fn count_non_blank_non_comment_lines(content: &str) -> usize {
    content.lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with("//") && !trimmed.starts_with("/*") && !trimmed.starts_with("*/")
        })
        .count()
}

fn analyze_modules(path: &Path) -> HashMap<String, HashMap<String, (usize, Vec<String>, Vec<(String, usize)>)>> {
    let mut module_structure = HashMap::new();
    let mod_regex = Regex::new(r"mod\s+(\w+)").unwrap();
    let function_regex = Regex::new(r"fn\s+(\w+)").unwrap();
    let struct_regex = Regex::new(r"struct\s+(\w+)").unwrap();
    let enum_regex = Regex::new(r"enum\s+(\w+)").unwrap();

    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        if entry.path().extension().map_or(false, |ext| ext == "rs") {
            let file_path = entry.path().strip_prefix(path).unwrap().to_str().unwrap().to_string();
            let content = fs::read_to_string(entry.path()).expect("Failed to read file");
            let line_count = content.lines().count();
            let mut functions = HashSet::new();
            let mut data_structures = Vec::new();

            for func_cap in function_regex.captures_iter(&content) {
                if let Some(func_name) = func_cap.get(1) {
                    functions.insert(func_name.as_str().to_string());
                }
            }

            for struct_cap in struct_regex.captures_iter(&content) {
                if let Some(name) = struct_cap.get(1) {
                    if let Some(item) = data_structures.iter_mut().find(|(n, _)| n == name.as_str()) {
                        item.1 += 1;
                    } else {
                        data_structures.push((name.as_str().to_string(), 1));
                    }
                }
            }

            for enum_cap in enum_regex.captures_iter(&content) {
                if let Some(name) = enum_cap.get(1) {
                    if let Some(item) = data_structures.iter_mut().find(|(n, _)| n == name.as_str()) {
                        item.1 += 1;
                    } else {
                        data_structures.push((name.as_str().to_string(), 1));
                    }
                }
            }

            for mod_cap in mod_regex.captures_iter(&content) {
                if let Some(mod_name) = mod_cap.get(1) {
                    let module_entry = module_structure.entry(mod_name.as_str().to_string()).or_insert_with(HashMap::new);
                    module_entry.insert(file_path.clone(), (line_count, functions.clone().into_iter().collect(), data_structures.clone()));
                }
            }
        }
    }

    module_structure
}

fn total_lines_in_module(module_structure: &HashMap<String, HashMap<String, (usize, Vec<String>, Vec<(String, usize)>)>>, module: &str) -> usize {
    if let Some(files) = module_structure.get(module) {
        files.values().map(|(line_count, _, _)| *line_count).sum()
    } else {
        0
    }
}