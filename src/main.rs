use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::process::Command;
use toml;

#[derive(Debug, Deserialize)]
struct PathsConfig {
    paths: std::collections::BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Commit {
    hash: String,
    message: String,
    author_name: String,
    author_email: String,
    date: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Project {
    name: String,
    commits: Vec<Commit>,
    remote: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ProjectList {
    projects: Vec<Project>,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: {} <author_name> <days>", args[0]);
        std::process::exit(1);
    }

    let author_name = &args[1];
    let days: i64 = args[2].parse().expect("Failed to parse the number of days");

    let config = read_config();
    let project_list = process_projects(&config, author_name, days);
    generate_changelog(&project_list);
}

fn get_remote(path: &str) -> String {
    let remote_command = Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("remote")
        .arg("get-url")
        .arg("origin")
        .output()
        .expect("failed to execute git remote get-url origin");

    let remote_command = String::from_utf8_lossy(&remote_command.stdout);
    let trimmed_remote_command = remote_command.trim();
    trimmed_remote_command.to_string()
}

fn get_log(path: &str, days: i64) -> String {
    let log_command = Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("log")
        .arg("--since")
        .arg(format!("{} days ago", days))
        .arg("--pretty=format:%H,%s,%an,%ae,%ad")
        .output()
        .expect("failed to execute git log");

    let log_command = String::from_utf8_lossy(&log_command.stdout);

    log_command.to_string()
}

fn read_config() -> PathsConfig {
    let config_str = fs::read_to_string("config.toml").expect("Failed to open config file");
    toml::from_str(&config_str).expect("Failed to parse config")
}

fn process_projects(config: &PathsConfig, author_name: &str, days: i64) -> ProjectList {
    let mut project_list = ProjectList { projects: vec![] };

    for (name, path) in &config.paths {
        let remote = get_remote(path);

        let mut project = Project {
            name: name.to_string(),
            commits: vec![],
            remote,
        };

        let log_command = get_log(path, days);

        for line in log_command.lines() {
            let commit: Vec<&str> = line.split(",").collect();
            if commit[2].to_string() == *author_name {
                let commit = Commit {
                    hash: commit[0].to_string(),
                    message: commit[1].to_string(),
                    author_name: commit[2].to_string(),
                    author_email: commit[3].to_string(),
                    date: commit[4].to_string(),
                };
                project.commits.push(commit);
            }
        }

        project_list.projects.push(project);
    }

    project_list
}

fn generate_changelog(projects: &ProjectList) {
    let mut changelog = String::new();

    changelog.push_str(&format!(
        "# Changelog for {}\n\n",
        Local::now().format("%Y-%m-%d")
    ));

    for project in &projects.projects {
        changelog.push_str(&format!("## {}\n", project.name));

        let (project_features, project_bug_fixes) = separate_features_and_bug_fixes(&project);

        if !project_bug_fixes.is_empty() {
            changelog.push_str("### :bug: Bugfixes\n");
            changelog.push_str(&project_bug_fixes);
        }

        if !project_features.is_empty() {
            changelog.push_str("### :rocket: Features\n");
            changelog.push_str(&project_features);
        }

        changelog.push('\n');
    }

    fs::write("changelog.md", changelog).expect("Failed to write changelog.md");
}

fn separate_features_and_bug_fixes(project: &Project) -> (String, String) {
    let mut project_features = String::new();
    let mut project_bug_fixes = String::new();

    for commit in &project.commits {
        let message_parts: Vec<&str> = commit.message.split(": ").collect();
        if message_parts.len() == 2 {
            let message: String = commit.message.split(": ").nth(1).unwrap().to_string();
            let commit_link = format!("{}/commits/{}", project.remote, commit.hash);
            if commit.message.starts_with("feat:") {
                project_features.push_str(&format!(
                    " - {} [#{}]({})\n",
                    message,
                    &commit.hash[0..8],
                    commit_link
                ));
            } else if commit.message.starts_with("fix:") {
                project_bug_fixes.push_str(&format!(
                    " - {} [#{}]({})\n",
                    message,
                    &commit.hash[0..8],
                    commit_link
                ));
            }
        }
    }

    (project_features, project_bug_fixes)
}
