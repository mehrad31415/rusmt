### Documentation

The project documentation is an [mdBook](https://rust-lang.github.io/mdBook/) under `book/`.
The `book/book.toml` file configures the build, and `book/src/` contains the markdown sources (rooted at `SUMMARY.md`). To build locally:

```bash
cd book
mdbook build
mdbook serve
```

Or just simply run `make docs`.