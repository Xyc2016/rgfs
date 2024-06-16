use clap::Parser;
use glob::PatternError;
use std::path::{self, PathBuf};
use std::process;

use log::{error, info, trace, warn};

#[derive(Debug, Parser)]
/// Transform staged files using a formatting command that accepts content via stdin and produces a result via stdout.
#[command(version, about)]
struct Cli {
    /// Shell command to format files, will run once per file. Occurrences of the placeholder `{}` will be replaced with a path to the file being formatted. (Example: "prettier --stdin-filepath "{}"")
    #[arg(short, long)]
    formatter: String,

    /// By default formatting changes made to staged file content will also be applied to working tree files via a patch. This option disables that behavior, leaving working tree files untouched.
    #[arg(long, action = clap::ArgAction::SetTrue)]
    no_update_working_tree: bool,

    /// Prevents rgfs from modifying staged or working tree files. You can use this option to check staged changes with a linter instead of formatting. With this option stdout from the formatter command is ignored. Example: rgfs --no-write -f "eslint --stdin --stdin-filename "{}" >&2" "*.js"
    #[arg(long, action = clap::ArgAction::SetTrue)]
    no_write: bool,

    /// Show the formatting commands that are running
    #[arg(long, action = clap::ArgAction::SetTrue)]
    verbose: bool,

    /// Patterns that specify files to format. The formatter will only transform staged files that are given here. Patterns may be literal file paths, or globs which will be tested against staged file paths using Python"s fnmatch function. For example "src/*.js" will match all files with a .js extension in src/ and its subdirectories. Patterns may be negated to exclude files using a "!" character. Patterns are evaluated left-to-right. (Example: "main.js" "src/*.js" "test/*.js" "!test/todo/*")
    #[arg(action = clap::ArgAction::Append)]
    files: Vec<String>,
}

fn get_git_root() -> PathBuf {
    let output = process::Command::new("git")
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output()
        .expect("Failed to run git rev-parse --show-toplevel");
    let git_root = std::str::from_utf8(&output.stdout)
        .expect("Failed to parse git rev-parse --show-toplevel output")
        .trim();
    PathBuf::from(git_root)
}

/*
'src_mode': unless_zeroed(m.group(1)),
'dst_mode': unless_zeroed(m.group(2)),
'src_hash': unless_zeroed(m.group(3)),
'dst_hash': unless_zeroed(m.group(4)),
'status': m.group(5),
'score': int(m.group(6)) if m.group(6) else None,
'src_path': m.group(7),
'dst_path': m.group(8)
}
*/
struct StagedFile<'a> {
    src_mode: &'a str,
    dst_mode: &'a str,
    src_hash: &'a str,
    dst_hash: &'a str,
    status: &'a str,
    score: Option<i32>,
    src_path: &'a str,
    dst_path: &'a str,
}

fn parse_diff<'a>(diff: &'a str) -> StagedFile<'a> {
    let re = regex::Regex::new(
        r"(\d{6}) (\d{6}) ([0-9a-f]{40}) ([0-9a-f]{40}) ([ACDMRTUXB])\d{0,3} (.+?)(?:\t(.+))?$",
    )
    .unwrap();
    let captures = re.captures(diff).expect("Failed to parse diff");
    StagedFile {
        src_mode: captures.get(1).unwrap().as_str(),
        dst_mode: captures.get(2).unwrap().as_str(),
        src_hash: captures.get(3).unwrap().as_str(),
        dst_hash: captures.get(4).unwrap().as_str(),
        status: captures.get(5).unwrap().as_str(),
        score: captures.get(6).map(|m| m.as_str().parse().unwrap()),
        src_path: captures.get(7).unwrap().as_str(),
        dst_path: captures.get(8).map(|m| m.as_str()).unwrap_or(""),
    }
}

fn normalize_path(relative_path: &str, git_root: Option<&PathBuf>) -> PathBuf {
    match git_root {
        Some(root) => root.join(relative_path),
        None => PathBuf::from(relative_path),
    }
}

fn matches_some_path(signed_patterns: &Vec<SignedPattern>, path: &PathBuf) -> bool {
    let path_str = match path.to_str() {
        Some(p) => p,
        None => {
            warn!("Failed to convert path to string: {:?}, skip.", path);
            return false;
        }
    };
    let mut is_match = false;

    for signed_pattern in signed_patterns {
        let SignedPattern(is_pattern_positive, pattern) = signed_pattern;
        if pattern.matches(path_str) {
            is_match = *is_pattern_positive;
        }
    }
    is_match
}

fn format_staged_files(
    signed_patterns: &Vec<SignedPattern>,
    formatter: &str,
    git_root: PathBuf,
    update_working_tree: bool,
    write: bool,
    verbose: bool,
) {
    let command_get_staged = process::Command::new("git")
        .args([
            "diff-index",
            "--cached",
            "--diff-filter=AM",
            "--no-renames",
            "HEAD",
        ])
        .output()
        .expect("Failed to run git diff-index --cached --diff-filter=AM --no-renames HEAD");
    let staged_files = std::str::from_utf8(&command_get_staged.stdout).expect(
        "Failed to parse git diff-index --cached --diff-filter=AM --no-renames HEAD output",
    );
    for line in staged_files.lines() {
        let entry = parse_diff(line);
        let entry_path = normalize_path(entry.src_path, Some(&git_root));
        if entry.dst_mode == "120000" {
            // Do not process symlinks
            if verbose {
                warn!("Skipping symlink: {}", entry_path.display());
            }
            continue;
        }
        if !matches_some_path(signed_patterns, &entry_path) {
            continue;
        }
        if format_file_in_index(formatter, entry, update_working_tree, write=write, verbose=verbose)  {
            info!("Reformatted {} with {}", entry.src_path, formatter)
        }
    }
}

fn format_file_in_index(
    formatter: &str,
    diff_entry: StagedFile,
    update_working_tree: bool,
    write: bool,
    verbose: bool,
) -> bool {
    let orig_hash = diff_entry.dst_hash;

    todo!("finish this function")
}

struct SignedPattern(bool, glob::Pattern);

impl SignedPattern {
    fn from_str(pattern: &str) -> Result<Self, PatternError> {
        if pattern.starts_with("!") {
            Ok(Self(false, glob::Pattern::new(&pattern[1..])?))
        } else {
            Ok(Self(true, glob::Pattern::new(pattern)?))
        }
    }
}

fn main() {
    let cli = Cli::parse();
    let git_root = get_git_root();
    let signed_patterns = cli
        .files
        .iter()
        .map(|pattern| match SignedPattern::from_str(pattern) {
            Ok(p) => p,
            Err(e) => {
                error!("Invalid file pattern: {}", e);
                process::exit(1);
            }
        })
        .collect();
    format_staged_files(
        &signed_patterns,
        &cli.formatter,
        git_root,
        !cli.no_update_working_tree,
        !cli.no_write,
        cli.verbose,
    );
}
