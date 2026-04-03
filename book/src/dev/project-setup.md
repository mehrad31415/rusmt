## Project Setup Guide

This page focuses on what you need to build/test the *current* repository state.

### Prerequisites

- **Rust** (edition 2024) with the stable toolchain
- **CMake** and a **C++ compiler** -- required to build the vendored Z3 dependency
  - macOS: included with Xcode Command Line Tools (`xcode-select --install`)
  - Ubuntu/Debian: `sudo apt install build-essential cmake`

The first `cargo build` compiles Z3 from source (~5 minutes). Subsequent builds use the cached result. No system Z3 binary is required.

### Workspace build

```bash
cargo build --workspace
```

### Make targets

The root `Makefile` provides a few convenience targets:

- `make lint`: `cargo fmt` + `cargo clippy`
- `make cloc`: count Rust LOC under `smt/`, `lang/`
- `make docs`: build and serve the mdBook under `book/`

### License

This project is licensed under the **GNU General Public License (GPL) Version 3**. The GPLv3 is a free, copyleft license that ensures the software remains free and open for all users. Key points include:

- **Freedom to Use, Modify, and Share**: You are free to use, modify, and distribute the software, as long as any derivative work is also licensed under the GPLv3 (copyleft). This ensures that the software remains free and open.
- **Source Code Availability**: If you distribute the software, you must also provide the source code, ensuring others can study and modify it.
- **No Warranty**: The developers are not liable for any issues arising from its use.

For more details, please refer to the `LICENSE` file included in this repository.

### Rust Toolchain

The project uses a `rust-toolchain` file to define the specific toolchain configuration. `rust-toolchain` in simple words is a pinning file that tells _rustup_ (the Rust version manager) which Rust version to use and which extra tools to install alongside it. When anyone runs `cargo build` or any Rust command inside this project folder, rustup automatically reads this file and switches to exactly that toolchain. This guarantees every developer and every CI machine uses the same Rust setup and no "works on my machine" issues. This file contains the following configuration:

```
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
```
- **Channel**: `stable` – This ensures that the project uses the latest stable version of the Rust compiler. To check the configuration of the Rust toolchain in the root directory you can run `rustup show` to verify the correct channel is active.

- **Components**: 
  - `rustfmt`: A tool that automatically formats Rust code according to standard style guidelines. Running `cargo fmt` will automatically improve code readability and reduce stylistic inconsistencies in the sourcecode.
  - `clippy`: A linting tool that catches common bugs, and suggests improvements, helping to maintain robustness in the source code. Running `cargo clippy` will lint the code and identify potential warning and errors.

These processes have been automated by the `make lint` command in the Makefile. Including `components = ["rustfmt", "clippy"]` in the `rust-toolchain` file ensures that anyone that clones the repository will automatically get these components.

### Gitignore

The `.gitignore` file specifies files and directories that should be ignored by Git. This helps prevent unnecessary files from being included in the repository when being pushed.

### Code Coverage 

`Coverage gutters` is an extension for Visual Studio Code that visualizes code coverage information in the editor. It searches for `lcov.info` files in the project root directory and visualizes the code coverage information inline. To generate the `lcov.info` file, you can use tools like `grcov` or `cargo tarpaulin`. The steps are as follows:

1. Install the `Coverage Gutters` extension in Visual Studio Code.
2. Install the `cargo-tarpaulin` crate by running `cargo install cargo-tarpaulin`.
3. Run `cargo tarpaulin --out Lcov` to generate the `lcov.info` file.

Depending on where we run the command `cargo tarpaulin --out Lcov`, the content of the `lcov.info` file will be different. If we have a workspace with multiple crates for example, the `lcov.info` file will always be created in the root of the workspace. However, if the command is run in the root of the workspace, the lcov.info file will contain coverage information for all the crates in the workspace. If the command is run in the directory of a specific crate, the lcov.info file will contain coverage information only for that specific crate. Now if we run the command `cargo tarpaulin --out Lcov --out html`, the coverage information will be generated in both lcov.info and html format. The html file will be named as `tarpaulin-report.html` by default. Note that the html file will always be created in the root of the workspace as well, unless specified by the `--output-dir` flag otherwise. To reiterate, depending on where the command is run, the content of the html file will be specific to that crate if the command is run in the directory of a specific crate; or the entire workspace if the command is run in the root of the workspace. In either way, all the crates will be listed in the html file, but the coverage information will be different. We can generate only the html file by running `cargo tarpaulin --out html`.

Now by openning the command pallet (cmd + shift + p in mac) and typing coverage, we can see the commands available from the `coverage gutters` extension. We have two important commands `watch/unwatch coverage`. The watch coverage will show which functions are covered and which are not by the test suite. The covered functions will have a green mark and the uncovered functions will have a red mark (you can customize the viewing in the extension settings. For example adding a show ruler/line/gutter option). Note that as we write new tests (or delete the previous ones), the inline display of the coverage in the editory is not automatically updated. To update the coverage information, we need to run the tarpaulin command again and then toggle the watch coverage command to see the new coverage information. The default version of cargo tarpaulin is `--ignore-tests`, which means that the coverage information will not be generated for the tests.

If we want to automatically update the coverage display inline in the editor whenever we write new tests, instead of running the tarpaulin command again and then toggling the watch coverage, we can do the following:

1. Install the `cargo-watch` tool by running `cargo install cargo-watch`.
2. Run the command `cargo watch -x 'tarpaulin --out Lcov' -i lcov.info` to automatically run the tarpaulin command whenever a file in the project changes.

`cargo-watch` is a command-line utility for Rust developers that automatically monitors changes in your project's files and executes specified commands whenever a change is detected. The `-x` flag tells cargo watch to execute what command whenever a change is detected. We can use this tool to run the tarpaulin command whenever a file in the project changes. This way, a new `Lcov` file will be automatically created whenever a source code is updated, and subsequently the inline coverage (provided by coverage gutters) will be updated as well. Note that coverage gutters provides the inline coverage in the editor by looking at the `Lcov` file. The `-i lcov.info` flag is used to ignore the lcov.info file when it changes. This is because the tarpaulin command will generate the lcov.info (in the root of the workspace by default) when it runs, so if we don't ignore it, the cargo watch command will run the tarpaulin command again when the lcov.info file changes, which will consequently create an infinite loop. So we need to ignore the lcov.info file when it changes. However, we can remove the -i lcov.info flag if the directory that we are watching and the directory where the lcov.info file is generated are different. For example, if we want to run the tarpaulin command inside a specific crate directory, because the lcov.info file will be created in the root of the workspace, it will not be in the directory of the crate, thus automatically ignoring it. To emphasize cargo watch only watches the files in the directory where it is run. However, the cargo tarpaulin will generate files in the root of the workspace by default. Lastly, instead of saying what to ignore, we can say what to include. For example, we can say `-w src/` to `watch` only the src directory. so the command will be `cargo watch -x 'tarpaulin --out Lcov' -w src`. The path must exist in the project. Note that the `-w` flag is only for the cargo watch command. The tarpaulin command will run for the entire workspace unless we specify otherwise. To have the lcov.info file generated in the target directory, we can use the `--output-dir` flag. For example, `cargo tarpaulin --out Lcov --output-dir target/tarpaulin`. This will generate the lcov.info file in the target/tarpaulin directory. Note that this directory already exists. This way we will have the file abstracted from the root of the workspace. Also whenever we run `cargo clean`, the target directory will be deleted, so the lcov.info file will be deleted as well.

The repository includes a `cov.sh` helper script at the workspace root for generating coverage reports (if you have a compatible setup/tooling installed).

### Cargo.toml

Rusmart is a Rust workspace. The top-level `Cargo.toml` lists workspace members and shared dependencies.
The current workspace members are:

- `smt/stdlib`
- `smt/remark` and `smt/remark/remark_derive`
- `smt/derive`
- `lang`

Z3 is included as a vendored Rust crate dependency (`z3 = { version = "0.20.0", features = ["vendored"] }` and `z3-sys = "0.11.0"`). The vendored build compiles Z3 from source on first build (~5 minutes) and requires CMake and a C++ compiler (included with Xcode on macOS, `build-essential` + `cmake` on Ubuntu). Subsequent builds use the cached result. No system Z3 installation or `$PATH` configuration is required.

### Cargo.lock

As mentioned earlier, the workspace has only one `Cargo.lock` file at the top level, rather than having a `Cargo.lock` in each crate’s directory. This ensures that all crates are using the same version of all dependencies. The `Cargo.lock` file is a file that Cargo generates to keep track of the exact versions of dependencies that are used in the project. The `Cargo.lock` file is automatically generated by Cargo when we run `cargo build` and is not meant to be edited manually.