FROM rust:1.83

# This doesn't actually *do* anything, but it prevents the `cargo install` command
# below from having to wait ~130 seconds each time for "Updating crates.io index"
# to run, since the result of *that* gets cached by this command.
RUN cargo search --limit 0

COPY . .

RUN cargo install --path .

ENTRYPOINT ["cargo-test-annotations"]
