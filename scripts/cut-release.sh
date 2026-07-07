#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  scripts/cut-release.sh --print-current-version
  scripts/cut-release.sh --print-next-version --version <semver>
  scripts/cut-release.sh --dry-run --version <semver> [--notes-file <path>]
  scripts/cut-release.sh --version <semver> --notes-file <path>

This crate uses SemVer from the root Cargo.toml. The next version is not
inferred because the repo has no checked-in bump policy; pass --version.
USAGE
}

dry_run=0
print_current=0
print_next=0
version=""
notes_file=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run)
      dry_run=1
      shift
      ;;
    --print-current-version)
      print_current=1
      shift
      ;;
    --print-next-version)
      print_next=1
      shift
      ;;
    --version)
      version="${2:-}"
      shift 2
      ;;
    --notes-file)
      notes_file="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

package_name="$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["packages"][0]["name"])')"
current_version="$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["packages"][0]["version"])')"

if [[ "$print_current" == 1 ]]; then
  echo "$current_version"
  exit 0
fi

if [[ "$print_next" == 1 ]]; then
  if [[ -z "$version" ]]; then
    echo "error: next-version inference is unsafe for this repo; pass --version <semver>" >&2
    exit 2
  fi
  echo "$version"
  exit 0
fi

if [[ -z "$version" ]]; then
  echo "error: --version <semver> is required; this repo does not infer the next SemVer bump" >&2
  exit 2
fi

if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([+-][0-9A-Za-z.-]+)?$ ]]; then
  echo "error: --version must be a SemVer value like 1.2.3" >&2
  exit 2
fi

tag="v$version"

if [[ "$dry_run" == 0 && -z "$notes_file" ]]; then
  echo "error: --notes-file is required for a real release" >&2
  exit 2
fi

if [[ -n "$notes_file" && ! -f "$notes_file" ]]; then
  echo "error: notes file not found: $notes_file" >&2
  exit 2
fi

if [[ "$version" == "$current_version" ]]; then
  echo "error: target version matches current version ($current_version)" >&2
  exit 2
fi

git fetch --tags origin
if git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
  echo "error: tag already exists: $tag" >&2
  exit 2
fi

default_branch="$(git symbolic-ref --short refs/remotes/origin/HEAD 2>/dev/null | sed 's#^origin/##')"
if [[ -z "$default_branch" ]]; then
  default_branch="$(git remote show origin | sed -n '/HEAD branch/s/.*: //p')"
fi
current_branch="$(git branch --show-current)"

if [[ "$dry_run" == 0 && "$current_branch" != "$default_branch" ]]; then
  echo "error: release must run from default branch ($default_branch); current branch is $current_branch" >&2
  exit 2
fi

if [[ "$dry_run" == 0 && -n "$(git status --porcelain)" ]]; then
  echo "error: working tree must be clean before cutting a release" >&2
  exit 2
fi

if [[ "$dry_run" == 1 ]]; then
  echo "Dry run for $package_name $current_version -> $version"
  if [[ "$current_branch" != "$default_branch" ]]; then
    echo "Warning: real release must run from default branch ($default_branch); current branch is $current_branch"
  fi
  if [[ -n "$(git status --porcelain)" ]]; then
    echo "Warning: real release requires a clean working tree"
  fi
  echo "Would update Cargo.toml and Cargo.lock"
  echo "Would run: cargo fmt -- --check"
  echo "Would run: cargo clippy --all-targets --all-features -- -D warnings"
  echo "Would run: cargo test"
  echo "Would commit: chore: release $tag"
  echo "Would tag: $tag"
  echo "Would push branch and tag to origin"
  if [[ -n "$notes_file" ]]; then
    echo "Would create GitHub release with notes from: $notes_file"
  else
    echo "Would require --notes-file before creating the GitHub release"
  fi
  exit 0
fi

python3 - "$version" <<'PY'
import pathlib
import re
import sys

version = sys.argv[1]
path = pathlib.Path("Cargo.toml")
text = path.read_text()
updated, count = re.subn(r'(?m)^version = "[^"]+"$', f'version = "{version}"', text, count=1)
if count != 1:
    raise SystemExit("error: could not find a single package version in Cargo.toml")
path.write_text(updated)
PY

cargo check
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test

git add Cargo.toml Cargo.lock
git commit -m "chore: release $tag"
git tag -a "$tag" -m "$tag"
git push origin "$current_branch"
git push origin "$tag"
gh release create "$tag" --title "$tag" --notes-file "$notes_file"
