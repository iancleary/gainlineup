#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

bin="$tmp/bin"
mkdir -p "$bin"
cat > "$bin/cargo" <<'SH'
#!/usr/bin/env bash
if [[ "${1:-}" == metadata ]]; then
  printf '{"packages":[{"name":"fixture","version":"0.23.0"}]}\n'
elif [[ "${1:-}" == clippy && "${FAIL_CLIPPY:-0}" == 1 ]]; then
  exit 77
fi
SH
cat > "$bin/gh" <<'SH'
#!/usr/bin/env bash
printf '%s\n' "$*" >> "$GH_CALLS"
SH
chmod +x "$bin/cargo" "$bin/gh"
export PATH="$bin:$PATH"
export GH_CALLS="$tmp/gh-calls"
notes="$tmp/notes.md"
printf 'fixture notes\n' > "$notes"

origin="$tmp/origin.git"
work="$tmp/work"
git init --bare --initial-branch=main "$origin" >/dev/null
git init --initial-branch=main "$work" >/dev/null
git -C "$work" config user.name 'Release Test'
git -C "$work" config user.email 'release-test@example.invalid'
mkdir -p "$work/scripts"
cp "$repo_root/scripts/cut-release.sh" "$work/scripts/cut-release.sh"
cat > "$work/Cargo.toml" <<'TOML'
[package]
name = "fixture"
version = "0.23.0"
TOML
printf 'fixture lock\n' > "$work/Cargo.lock"
git -C "$work" add Cargo.toml Cargo.lock scripts/cut-release.sh
git -C "$work" commit -m 'fixture' >/dev/null
initial_head="$(git -C "$work" rev-parse HEAD)"
git -C "$work" remote add origin "$origin"
git -C "$work" push -u origin main >/dev/null
git -C "$work" remote set-head origin main >/dev/null

if (cd "$work" && FAIL_CLIPPY=1 bash scripts/cut-release.sh --version 0.23.0 --notes-file "$notes") >"$tmp/check-failure.out" 2>&1; then
  echo 'expected injected release-check failure to stop release' >&2
  exit 1
fi
[[ "$(git -C "$work" rev-parse HEAD)" == "$initial_head" ]]
! git -C "$work" rev-parse -q --verify refs/tags/v0.23.0 >/dev/null
! git --git-dir="$origin" rev-parse -q --verify refs/tags/v0.23.0 >/dev/null
[[ ! -f "$GH_CALLS" ]]

dry_run_output="$(cd "$work" && bash scripts/cut-release.sh --dry-run --version 0.23.0 --notes-file "$notes")"
[[ "$dry_run_output" == *"without changing version files or creating a commit"* ]]
[[ "$(git -C "$work" rev-parse HEAD)" == "$initial_head" ]]
[[ -z "$(git -C "$work" status --porcelain)" ]]

(cd "$work" && bash scripts/cut-release.sh --version 0.23.0 --notes-file "$notes")
[[ "$(git -C "$work" rev-parse HEAD)" == "$initial_head" ]]
[[ "$(git -C "$work" rev-parse 'refs/tags/v0.23.0^{commit}')" == "$initial_head" ]]
[[ "$(git --git-dir="$origin" rev-parse 'refs/tags/v0.23.0^{commit}')" == "$initial_head" ]]
grep -q '^release create v0.23.0 --title v0.23.0 --notes-file ' "$GH_CALLS"

if (cd "$work" && bash scripts/cut-release.sh --version 0.23.0 --notes-file "$notes") >"$tmp/tag.out" 2>&1; then
  echo 'expected existing-tag guard to fail' >&2
  exit 1
fi
grep -q 'tag already exists: v0.23.0' "$tmp/tag.out"

git -C "$work" checkout -b wrong-branch >/dev/null
if (cd "$work" && bash scripts/cut-release.sh --version 0.23.1 --notes-file "$notes") >"$tmp/branch.out" 2>&1; then
  echo 'expected default-branch guard to fail' >&2
  exit 1
fi
grep -q 'release must run from default branch (main)' "$tmp/branch.out"

git -C "$work" checkout main >/dev/null
printf 'dirty\n' >> "$work/Cargo.toml"
if (cd "$work" && bash scripts/cut-release.sh --version 0.23.1 --notes-file "$notes") >"$tmp/dirty.out" 2>&1; then
  echo 'expected clean-tree guard to fail' >&2
  exit 1
fi
grep -q 'working tree must be clean' "$tmp/dirty.out"

echo 'release runner current-version and safety-guard checks passed'
