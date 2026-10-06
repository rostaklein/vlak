# Play VLAK

**[Download vlak.exe](vlak.exe?raw=true)** — 64-bit Windows 10 / 11.

Save the file and double-click it. No installation or extra game files are
needed. Czech is the default; `L` switches Czech / English. Arrow keys steer,
Space pauses, `R` restarts, and `H` opens help.

This is the Rust recreation of Miroslav Němeček's 1993 game, with all 50 original
levels and graphics restored from archival references.
[Full instructions and credits](../README.md) · [License](../LICENSE) ·
[Asset provenance](../assets/README.md)

The binary's SHA-256 is listed in [SHA256SUMS](SHA256SUMS). To inspect your copy:

```powershell
Get-FileHash .\vlak.exe -Algorithm SHA256
```

Maintainers: run `powershell -File .\scripts\package.ps1` from the repository
root to rebuild this executable and refresh its checksum before committing.
