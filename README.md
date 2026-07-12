# Hamrah

A content-addressed folder sync tool that keeps a directory identical across all your machines, built on git's content model. A "Dropbox for developers."

The name *hamrah* (ham-raah) is Persian (همراه) for "companion; one who travels with you."

> 🚧 Early and in progress. Bidirectional sync now works end-to-end over TCP.

## Status

- [x] Content-addressed store (`hash → blob`)
- [x] Manifest (`path → hash`)
- [x] Length-prefixed wire protocol
- [x] Async TCP transport
- [x] One-way sync (writes real files)
- [x] File watcher (auto-sync on change)
- [x] Bidirectional sync + diff
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
