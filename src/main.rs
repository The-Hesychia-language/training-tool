//! hsy-train: exercises to learn Hesychia. An exercise is a file to fix or to complete; it is
//! done when it builds, writes what is expected, and passes its checks.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::thread::sleep;
use std::time::{Duration, SystemTime};

const HELP: &str = "hsy-train: learn Hesychia by fixing and building small programs

    hsy-train          checks the current exercise each time its file is saved
    hsy-train list     the exercises, and which are done
    hsy-train hint     a hint for the current exercise
    hsy-train check    checks the current exercise once
    hsy-train verify   for authors: every exercise fails as given, and its solution passes

Run it in the training-tool folder. `hsy` must be installed; HSY names another compiler.";

/// An exercise: `exercises/<name>.hsy`, checked by `checks/<name>.hsy`.
struct Exercise {
    /// Chapter and file without extension: `01_basics/01_hello`.
    name: String,
}

impl Exercise {
    fn path(&self) -> PathBuf {
        Path::new("exercises").join(format!("{}.hsy", self.name))
    }

    fn solution(&self) -> PathBuf {
        Path::new("solutions").join(format!("{}.hsy", self.name))
    }

    /// The checks: the output `main` must write (`# expect:` lines), hints (`# hint:` lines),
    /// and `@test` functions.
    fn checks(&self) -> Result<String, String> {
        read(&Path::new("checks").join(format!("{}.hsy", self.name)))
    }
}

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

/// The lines of `text` after `marker`, such as `# expect: `.
fn marked<'a>(text: &'a str, marker: &str) -> Vec<&'a str> {
    text.lines().filter_map(|line| line.strip_prefix(marker)).collect()
}

/// Every exercise, in order: chapters, then files, by name.
fn exercises() -> Result<Vec<Exercise>, String> {
    let sorted = |dir: &Path| -> Result<Vec<PathBuf>, String> {
        let entries = fs::read_dir(dir).map_err(|_| {
            "no `exercises` folder here: run hsy-train in the training-tool folder".to_string()
        })?;
        let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
        paths.sort();
        Ok(paths)
    };
    let mut found = Vec::new();
    for chapter in sorted(Path::new("exercises"))? {
        if !chapter.is_dir() {
            continue;
        }
        for file in sorted(&chapter)? {
            if file.extension().is_some_and(|e| e == "hsy") {
                let chapter = chapter.file_name().unwrap_or_default().to_string_lossy();
                let stem = file.file_stem().unwrap_or_default().to_string_lossy();
                found.push(Exercise { name: format!("{chapter}/{stem}") });
            }
        }
    }
    Ok(found)
}

const PROGRESS: &str = ".progress";

/// The names of the exercises done.
fn done() -> HashSet<String> {
    fs::read_to_string(PROGRESS).unwrap_or_default().lines().map(str::to_string).collect()
}

fn mark_done(name: &str) -> Result<(), String> {
    let mut names = done();
    if names.insert(name.to_string()) {
        let mut names: Vec<String> = names.into_iter().collect();
        names.sort();
        fs::write(PROGRESS, names.join("\n") + "\n").map_err(|e| format!("{PROGRESS}: {e}"))?;
    }
    Ok(())
}

fn hsy() -> String {
    std::env::var("HSY").unwrap_or_else(|_| "hsy".to_string())
}

/// Runs `hsy` in `dir`; its output, and whether it succeeded.
fn run_hsy(dir: &Path, args: &[&str]) -> Result<(bool, String, String), String> {
    let output = Command::new(hsy())
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| format!("`{}` cannot be run ({e}): install Hesychia, or set HSY", hsy()))?;
    let text = |bytes: &[u8]| String::from_utf8_lossy(bytes).replace("\r\n", "\n");
    Ok((output.status.success(), text(&output.stdout), text(&output.stderr)))
}

/// Checks `source` as the answer to `exercise`: `Ok(Err(why))` when it is not one yet.
///
/// The source and its checks are built as one file, in a folder of their own, so that the
/// compiler's messages give the lines of the exercise.
fn check(exercise: &Exercise, source: &Path) -> Result<Result<(), String>, String> {
    let checks = exercise.checks()?;
    let dir = std::env::temp_dir().join("hsy-train").join(exercise.name.replace('/', "-"));
    fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let file = "exercise.hsy";
    let tests: String = checks
        .lines()
        .filter(|l| !l.starts_with("# hint: ") && !l.starts_with("# expect: "))
        .collect::<Vec<_>>()
        .join("\n");
    let whole = format!("{}\n{tests}\n", read(source)?.trim_end());
    fs::write(dir.join(file), whole).map_err(|e| format!("{}: {e}", dir.display()))?;
    let shown = |text: &str| text.replace(file, &source.display().to_string());

    let expected = marked(&checks, "# expect: ");
    if !expected.is_empty() {
        let (ok, out, err) = run_hsy(&dir, &["run", file])?;
        if !ok {
            return Ok(Err(shown(&format!("{out}{err}"))));
        }
        let written: Vec<&str> = out.lines().collect();
        if written != expected {
            let list = |lines: &[&str]| lines.iter().map(|l| format!("    {l}\n")).collect::<String>();
            return Ok(Err(format!(
                "{}The program runs, but it should write:\n{}It wrote:\n{}",
                shown(&err),
                list(&expected),
                list(&written)
            )));
        }
    }
    if checks.contains("@test") {
        let (ok, out, err) = run_hsy(&dir, &["test", file])?;
        if !ok {
            // Warnings would bury which checks failed; errors say why nothing was built.
            let errors = if err.lines().any(|l| l.starts_with("error")) { err.as_str() } else { "" };
            return Ok(Err(shown(&format!("{errors}{out}"))));
        }
    }
    Ok(Ok(()))
}

fn current(all: &[Exercise]) -> Option<&Exercise> {
    let done = done();
    all.iter().find(|e| !done.contains(&e.name))
}

fn modified(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn watch(all: &[Exercise]) -> Result<(), String> {
    loop {
        let Some(exercise) = current(all) else {
            println!("\nEvery exercise is done. Well played!");
            return Ok(());
        };
        let path = exercise.path();
        let count = all.iter().filter(|e| done().contains(&e.name)).count();
        println!("\n[{}/{}] {}", count + 1, all.len(), path.display());
        let seen = modified(&path);
        match check(exercise, &path)? {
            Ok(()) => {
                mark_done(&exercise.name)?;
                println!("✓ done");
            }
            Err(why) => {
                println!("{}", why.trim_end());
                println!(
                    "\nEdit {} and save it; `hsy-train hint` gives a hint. Ctrl-C stops.",
                    path.display()
                );
                while modified(&path) == seen {
                    sleep(Duration::from_millis(300));
                }
            }
        }
    }
}

fn list(all: &[Exercise]) {
    let done = done();
    let now = current(all).map(|e| e.name.as_str());
    for exercise in all {
        let mark = if done.contains(&exercise.name) {
            "✓"
        } else if now == Some(exercise.name.as_str()) {
            "→"
        } else {
            " "
        };
        println!("{mark} {}", exercise.name);
    }
    println!("{} of {} done", all.iter().filter(|e| done.contains(&e.name)).count(), all.len());
}

/// Every exercise must fail as it is given, and pass with its solution.
fn verify(all: &[Exercise]) -> Result<bool, String> {
    let mut good = true;
    for exercise in all {
        let given = check(exercise, &exercise.path())?;
        let solved = check(exercise, &exercise.solution())?;
        let problem = match (given, solved) {
            (Ok(()), _) => Some("passes as it is given".to_string()),
            (_, Err(why)) => Some(format!("its solution fails:\n{why}")),
            _ => None,
        };
        match problem {
            Some(problem) => {
                good = false;
                println!("FAIL  {}: {problem}", exercise.name);
            }
            None => println!("ok    {}", exercise.name),
        }
    }
    Ok(good)
}

fn run(command: Option<&str>) -> Result<bool, String> {
    if matches!(command, Some("help" | "--help" | "-h")) {
        println!("{HELP}");
        return Ok(true);
    }
    let all = exercises()?;
    match command {
        None => watch(&all)?,
        Some("list") => list(&all),
        Some("hint") => match current(&all) {
            Some(exercise) => {
                println!("{}", exercise.path().display());
                for hint in marked(&exercise.checks()?, "# hint: ") {
                    println!("  {hint}");
                }
            }
            None => println!("Every exercise is done."),
        },
        Some("check") => match current(&all) {
            Some(exercise) => match check(exercise, &exercise.path())? {
                Ok(()) => {
                    mark_done(&exercise.name)?;
                    println!("✓ {}", exercise.name);
                }
                Err(why) => {
                    println!("{}", why.trim_end());
                    return Ok(false);
                }
            },
            None => println!("Every exercise is done."),
        },
        Some("verify") => return verify(&all),
        Some(other) => return Err(format!("unknown command `{other}`\n\n{HELP}")),
    }
    Ok(true)
}

fn main() -> ExitCode {
    let command = std::env::args().nth(1);
    match run(command.as_deref()) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}
