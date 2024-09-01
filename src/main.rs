use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use regex::Regex;
use walkdir::WalkDir;
use std::collections::HashMap;
use std::collections::HashSet;
use tempfile::tempdir;


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
    let workspace_path = temp_dir.path().join(workspace_name);
    let (file_count, line_counts) = count_files_and_lines(&workspace_path);
    let module_structure = analyze_modules(&workspace_path);

    // Print results
    println!("Total Rust source files: {}", file_count);
    println!("Line counts per file:");
    for (file, count) in line_counts {
        println!("  {}: {} lines", file, count);
    }
    println!("\nModules and their functions/data structures:");
    for (module, files) in module_structure {
        println!("Module: {}", module);
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
}

fn clone_repo(url: &str, path: &Path) {
    Command::new("git")
        .args(&["clone", url, path.to_str().unwrap()])
        .output()
        .expect("Failed to clone repository");
}

fn count_files_and_lines(path: &Path) -> (usize, Vec<(String, usize)>) {
    let mut file_count = 0;
    let mut line_counts = Vec::new();

    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        if entry.path().extension().map_or(false, |ext| ext == "rs") {
            file_count += 1;
            let file_path = entry.path().strip_prefix(path).unwrap().to_str().unwrap().to_string();
            let content = fs::read_to_string(entry.path()).expect("Failed to read file");
            let line_count = content.lines().count();
            line_counts.push((file_path, line_count));
        }
    }

    (file_count, line_counts)
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