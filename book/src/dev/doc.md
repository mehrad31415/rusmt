### Documentation

The project documentation is an [mdBook](https://rust-lang.github.io/mdBook/) under `book/`.
The `book/book.toml` file configures the build, and `book/src/` contains the markdown sources (rooted at `SUMMARY.md`).

The current `book.toml` looks like:

```
[book]
src = "src"
title = "Rusmart Framework"
authors = ["Meng Xu", "Mehrad Haghshenas"]
language = "en"
multilingual = false

[build]
create-missing = false

[rust]
edition = "2021"

[output.html.playground]
runnable = true
```

To build locally:

```bash
cd book
mdbook build
mdbook serve
```