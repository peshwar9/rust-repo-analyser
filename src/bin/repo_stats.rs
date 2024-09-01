use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;
use regex::Regex;


fn main() {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);
    if args.len() != 3 {
        eprintln!("Usage: {} <repository_url> <workspace_name>", args[0]);
        return;
    }

    let repo_url = &args[1];
    let workspace_name = &args[2];
    let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory");
    println!(
        "Cloning repository from {} into {}",
        repo_url,
        temp_dir.path().display()
    );

    // Clone the repository
    clone_repo(repo_url, temp_dir.path());

    // Analyze the repository
    let workspace_path = temp_dir.path().join(workspace_name).join("src");
    println!("workspace_path: {}", workspace_path.display());
    let (file_count, line_counts) = count_files_and_lines(&workspace_path);
    println!("Total Rust source files: {}", file_count);
    let total_line_count = line_counts.iter().map(|(_, count)| count).sum::<usize>();
    println!(
        "Total non-blank and non-commented lines: {}",
        total_line_count
    );
    for (file_path, line_count) in line_counts {
        println!("{}:{}", file_path, line_count)
    }
    let struct_declarations = extract_struct_declarations(&workspace_path);
    for (file_path, declaration) in struct_declarations {
        println!("{}: {}", file_path, declaration);
    }

    //println!("Total non-blank and non-commented lines: {}", line_count);
}

fn clone_repo(repo_url: &str, path: &Path) {
    Command::new("git")
        .args(["clone", repo_url, path.to_str().unwrap()])
        .status()
        .expect("Failed to clone repository");
}

fn count_files_and_lines(path: &Path) -> (usize, Vec<(String, usize)>) {
    let mut file_count = 0;
    let mut line_counts = Vec::new();

    for dir_entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        if dir_entry
            .path()
            .extension()
            .map_or(false, |ext| ext == "rs")
        {
            file_count += 1;
            let file_path = dir_entry
                .path()
                .strip_prefix(path)
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();
            let content = fs::read_to_string(dir_entry.path()).expect("Failed to read file");
            //let line_count = content.lines().count();
            let line_count = content
                .lines()
                .filter(|line| {
                    let trimmed = line.trim();
                    !trimmed.is_empty() && !trimmed.starts_with("//")
                })
                .count();
            line_counts.push((file_path, line_count));
        }
    }
    (file_count, line_counts)
}

fn extract_struct_declarations(path: &Path) -> Vec<(String, String)> {
    let mut struct_declarations = Vec::new();
    let struct_regex = Regex::new(r"(?s)struct\s+(\w+)\s*\{[^}]*\}").unwrap();

    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        if entry.path().extension().map_or(false, |ext| ext == "rs") {
            let file_path = entry.path().strip_prefix(path).unwrap().to_str().unwrap().to_string();
            let content = fs::read_to_string(entry.path()).expect("Failed to read file");

            for captures in struct_regex.captures_iter(&content) {
                let struct_name = captures.get(1).unwrap().as_str().to_string();
                let struct_declaration = captures.get(0).unwrap().as_str().to_string();
                struct_declarations.push((file_path.clone(), struct_declaration));
            }
        }
    }

    struct_declarations
}