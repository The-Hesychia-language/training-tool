# hsy-train

Exercises to learn the [Hesychia](https://github.com/The-Hesychia-language/Hesychia) language, in the terminal. Each one is a small file, of one of two kinds:

- **FIX**: a program that does not build, or gives the wrong answer. The compiler's message, or the failing check, says where to look.
- **BUILD**: a problem to solve. The file states it and gives the function to complete; checks you do not see decide whether it is right.

## Install

Hesychia must be installed first ([its README](https://github.com/The-Hesychia-language/Hesychia#install) says how), which also brings Rust.

```sh
git clone https://github.com/The-Hesychia-language/training-tool.git
cd training-tool
cargo install --path .
```

## Use

Run `hsy-train` in the `training-tool` folder, and open the file it names in your editor. Each time you save, the exercise is checked again; once it passes, the next one comes.

```
Progress: [######>-----------------------------------------------------]  3/26
Current exercise: exercises/01_basics/04_fizzbuzz.hsy

h:hint / l:list / c:check all / x:reset / q:quit ?
```

| Key | What it does |
| --- | --- |
| `h` | A hint for the current exercise; again for the next hint, when there is one |
| `l` | The exercises, and which are done |
| `c` | Checks every exercise again: those that pass are done, the others are not |
| `x` | Puts the current exercise's file back as it was given (with `git checkout`) |
| `q` | Quits; progress is kept in `.progress` |

Stuck? `solutions/` holds one answer to each exercise.

## Chapters

| Chapter | What it covers |
| --- | --- |
| `01_basics` | Imports, variables, functions, conditions |
| `02_collections` | Vectors, comprehensions, dicts |
| `03_structs_enums` | Structs and methods, enums and `match` |
| `04_errors` | `Optional`, `try` / `except`, `raise` |
| `05_manual` | The `manual` level: annotations, borrows, moves |
| `06_traits_generics` | Traits, bounds, `Show` |
| `07_threads` | `spawn`, `parallel for` and atomics, channels |
| `08_stdlib` | `re`, files, `json` |

## Adding an exercise

An exercise is three files of the same name, such as `03_structs_enums/02_enums.hsy`; chapters and exercises come in the order of their names.

| File | Role |
| --- | --- |
| `exercises/<name>.hsy` | What the learner gets: a comment that teaches and states the task, and code to fix or complete |
| `solutions/<name>.hsy` | One answer |
| `checks/<name>.hsy` | `# hint:` lines; `# expect:` lines, the output `main` must write; `@test` functions, built together with the exercise and run by `hsy test` |

`hsy-train verify` (and `cargo test`) checks that every exercise fails as it is given and passes with its solution. `HSY` names another compiler than the `hsy` on the `PATH`.
