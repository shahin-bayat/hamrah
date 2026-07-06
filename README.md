# Hamrah

A content-addressed folder sync tool — keep a directory identical across your machines, built on git's content model. "Dropbox for developers."

Name: *hamrah* (ham-raah), Persian همراه — "companion; travels with you."

> 🚧 Early and in progress — one-way sync works end-to-end.

## Status

- [x] Content-addressed store (`hash → blob`)
- [x] Manifest (`path → hash`)
- [x] Length-prefixed wire protocol
- [x] Async TCP transport
- [x] One-way sync (writes real files)
- [x] File watcher (auto-sync on change)
- [ ] Bidirectional sync + diff
- [ ] TLS + peer auth
- [ ] Multi-peer + discovery
- [ ] Lazy FUSE mount
- [ ] Conflict resolution / versioning

## Usage

```sh
cargo build --release

# receiver (listens):
hamrah receive 0.0.0.0:9000 /path/to/dir

# sender:
hamrah send <receiver-ip>:9000 /path/to/dir
```

## License

MIT
