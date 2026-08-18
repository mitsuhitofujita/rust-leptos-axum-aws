# CLAUDE.md

## Environment

This project runs inside a dev container.

- Python is not available. Do not propose or introduce scripts or tooling that
  depend on it.
- When a helper tool is needed, write it in Rust and keep it reusable.
- Recipes in the `justfile` are the exception: bash is allowed there. It is the
  task runner's own idiom, and a crate with a build step in front of a few shell
  lines is worse than the lines.
- ripgrep (`rg`) is available. Use it for searching.

## Working in the repository

The project lives inside the dev container and is under git control, so changes
can be made without worrying about affecting the host system. Building,
deleting, and regenerating files may proceed without asking first.
