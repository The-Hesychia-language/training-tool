//! hsy-train: exercises to learn Hesychia. An exercise is a file to fix or to complete; it is
//! done when it builds, writes what is expected, and passes its checks.

use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::thread::sleep;
use std::time::{Duration, SystemTime};

use crossterm::cursor::MoveTo;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{self, Clear, ClearType};

const HELP: &str = "hsy-train: learn Hesychia by fixing and building small programs

    hsy-train          the exercises: each is checked again when its file is saved
    hsy-train verify   for authors: every exercise fails as given, and its solution passes

Run it in the training-tool folder. `hsy` must be installed; HSY names another compiler.";

const KEYS: &str = "h:hint / l:list / c:check all / x:reset / q:quit ? ";

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

fn save_done(names: &HashSet<String>) -> Result<(), String> {
    let mut names: Vec<&str> = names.iter().map(String::as_str).collect();
    names.sort();
    fs::write(PROGRESS, names.join("\n") + "\n").map_err(|e| format!("{PROGRESS}: {e}"))
}

fn mark_done(name: &str) -> Result<(), String> {
    let mut names = done();
    names.insert(name.to_string());
    save_done(&names)
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
    // A failure names its line in the test, which the learner does not see: only what the
    // compiler says about the exercise's own lines is kept.
    let given = read(source)?.trim_end().lines().count();
    let shown = |text: &str| {
        let hidden = |line: &str| {
            line.strip_prefix(&format!("      at {file}:"))
                .is_some_and(|n| n.parse().is_ok_and(|n: usize| n > given))
        };
        let kept: Vec<&str> = text.lines().filter(|line| !hidden(line)).collect();
        kept.join("\n").replace(file, &source.display().to_string()) + "\n"
    };

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

/// A key pressed within a short wait, if the terminal gives keys one by one.
fn key() -> Option<char> {
    let wait = Duration::from_millis(300);
    if terminal::enable_raw_mode().is_err() {
        sleep(wait);
        return None;
    }
    let mut pressed = None;
    if event::poll(wait).unwrap_or(false)
        && let Ok(Event::Key(k)) = event::read()
        && k.kind == KeyEventKind::Press
        && let KeyCode::Char(c) = k.code
    {
        // Ctrl-C does not interrupt while keys are read one by one: it quits.
        pressed = Some(if k.modifiers.contains(KeyModifiers::CONTROL) && c == 'c' { 'q' } else { c });
    }
    let _ = terminal::disable_raw_mode();
    pressed
}

/// `Progress: [####>-----]  4/26`
fn progress_bar(count: usize, total: usize) -> String {
    const WIDTH: usize = 60;
    let filled = (count * WIDTH).checked_div(total).unwrap_or(0);
    let bar: String = (0..WIDTH)
        .map(|i| match i {
            i if i < filled => '#',
            i if i == filled => '>',
            _ => '-',
        })
        .collect();
    format!("Progress: [{bar}]  {count}/{total}")
}

fn listing(all: &[Exercise]) -> String {
    let done = done();
    let now = current(all).map(|e| e.name.as_str());
    let mut out = String::new();
    for exercise in all {
        let mark = if done.contains(&exercise.name) {
            "✓"
        } else if now == Some(exercise.name.as_str()) {
            "→"
        } else {
            " "
        };
        out += &format!("{mark} {}\n", exercise.name);
    }
    out
}

/// Checks every exercise again: those that pass are done, the others are not.
fn check_all(all: &[Exercise]) -> Result<(), String> {
    let mut passed = HashSet::new();
    for (i, exercise) in all.iter().enumerate() {
        print!("\rChecking {}/{}", i + 1, all.len());
        let _ = io::stdout().flush();
        if check(exercise, &exercise.path())?.is_ok() {
            passed.insert(exercise.name.clone());
        }
    }
    println!();
    save_done(&passed)
}

/// Puts an exercise's file back as it was given, from the repository.
fn reset(exercise: &Exercise) -> String {
    let path = exercise.path();
    let restored = Command::new("git").arg("checkout").arg("--").arg(&path).status();
    match restored {
        Ok(status) if status.success() => format!("{} is as it was given.", path.display()),
        _ => format!("{} could not be reset: `git checkout` failed.", path.display()),
    }
}

/// What the learner sees: the last result, a note (a hint, the list), then where they are.
fn show(result: &str, note: &str, all: &[Exercise], exercise: Option<&Exercise>) {
    let _ = execute!(io::stdout(), Clear(ClearType::All), MoveTo(0, 0));
    for part in [result, note] {
        if !part.is_empty() {
            println!("{}\n", part.trim_end());
        }
    }
    let count = all.iter().filter(|e| done().contains(&e.name)).count();
    println!("{}", progress_bar(count, all.len()));
    match exercise {
        Some(exercise) => {
            println!("Current exercise: {}\n", exercise.path().display());
            print!("{KEYS}");
        }
        None => println!("\nEvery exercise is done. Well played!"),
    }
    let _ = io::stdout().flush();
}

fn watch(all: &[Exercise]) -> Result<(), String> {
    let mut note = String::new();
    loop {
        let Some(exercise) = current(all) else {
            show("", &note, all, None);
            return Ok(());
        };
        let path = exercise.path();
        let seen = modified(&path);
        let result = match check(exercise, &path)? {
            Ok(()) => {
                mark_done(&exercise.name)?;
                note = format!("✓ {} is done.", path.display());
                continue;
            }
            Err(why) => why,
        };
        show(&result, &note, all, Some(exercise));
        let hints = exercise.checks()?;
        let hints = marked(&hints, "# hint: ");
        let mut given = 0;
        while modified(&path) == seen {
            match key() {
                Some('h') => {
                    given = (given + 1).min(hints.len());
                    note = hints[..given].iter().map(|h| format!("Hint: {h}\n")).collect();
                    show(&result, &note, all, Some(exercise));
                }
                Some('l') => {
                    note = listing(all);
                    show(&result, &note, all, Some(exercise));
                }
                Some('c') => {
                    println!();
                    check_all(all)?;
                    note = "Every exercise was checked again.".to_string();
                    break;
                }
                Some('x') => {
                    note = reset(exercise);
                    break;
                }
                Some('q') => {
                    println!();
                    return Ok(());
                }
                _ => {}
            }
        }
        if modified(&path) != seen {
            note.clear();
        }
    }
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
