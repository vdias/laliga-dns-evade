# Claude

Personal repository for everything Claude-related: skills vendored from
third-party sources, reference docs, and any other configuration worth
keeping in sync across machines.

## Layout

- `skills/<name>/` — vendored copy of a third-party skill (no `.git`, tracked
  as plain files in this repo).
- `docs/` — notes, procedures, or configuration write-ups.
- `sources.json` — manifest of upstream repos to track (name, URL, branch).
- `scripts/Update-Skills.ps1` — for each entry in `sources.json`: clones it
  if not present locally, updates it if present, vendors the result into
  `skills/<name>/`, rebuilds `zips/<name>.zip` only if something changed (or
  the ZIP doesn't exist yet), and commits + pushes any changes to `skills/`
  to this repo. `zips/` is gitignored — it is a local build artifact, not
  source of truth.

## Naming convention

Repo name (GitHub and local folder): `claude`, all lowercase. Local path
convention across machines: `C:\git\personal\claude`. Lowercase avoids any
case-sensitivity mismatch between Windows, GitHub, and any future WSL/Linux
use, since Windows filesystems are case-preserving but case-insensitive.

## First-time setup on a new machine

1. Install Git for Windows if not already present.
2. (One-time per machine) Enable long path support, in case any vendored
   skill ever has deeply nested paths:
   ```
   git config --global core.longpaths true
   ```
3. Clone this repo at the agreed path (SSH):
   ```
   git clone git@github.com:vdias/claude.git C:\git\personal\claude
   Set-Content -Path "README.md" -Value "# Claude Skills Repository"
   git add .
   git commit -m "feat: initial setup for claude skills management and automation scripts"
   git push -u origin main
   ```
4. Run `scripts\Update-Skills.ps1` to sync all sources and produce
   upload-ready ZIPs locally.

## Day-to-day usage

- **Getting the latest skills, on any machine:** run
  `scripts\Update-Skills.ps1`. It clones anything new, updates anything that
  already exists, rebuilds only the ZIPs that actually changed, and commits
  + pushes any changes to `skills/` to this repo.
- **Getting updates that were already pushed from another machine:**
  `git pull`, then run `scripts\Update-Skills.ps1` again so local ZIPs
  reflect what's now in `skills/`.
- **Adding a new skill to track:** add an entry to `sources.json`, then run
  `scripts\Update-Skills.ps1`.
- **Adding your own docs/config, not from an upstream repo:** just add files
  under `docs/` (or elsewhere in the repo) and commit/push normally — the
  script only manages the `skills/` and `zips/` folders.

## Notes

- Keep this repository **private** if it will ever contain anything specific
  to a work environment (internal procedures, naming conventions, etc.).
- Vendored skills under `skills/` come from third parties. Review changes
  (`git log`, `git diff`) before re-uploading an updated ZIP to claude.ai.
