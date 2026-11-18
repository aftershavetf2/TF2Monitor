"""Bump the TF2Monitor patch version and push its release tag to origin."""

import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "Cargo.toml"
LOCKFILE = ROOT / "Cargo.lock"
MANIFEST_VERSION = re.compile(
    rb'^version[ \t]*=[ \t]*"([0-9]+\.[0-9]+\.[0-9]+)"[ \t]*\r?$', re.MULTILINE
)
LOCK_VERSION = re.compile(
    rb'^name = "tf2monitor"\r?\nversion = "([^"]+)"', re.MULTILINE
)


def git(*args, capture=True):
    return subprocess.run(
        ["git", *args], cwd=ROOT, check=True, text=True, capture_output=capture
    ).stdout


def match_version(pattern, contents, name):
    matches = list(pattern.finditer(contents))
    if len(matches) != 1:
        raise ValueError(f"Expected one TF2Monitor version in {name}.")
    return matches[0]


def replace_version(contents, match, version):
    start, end = match.span(1)
    return contents[:start] + version.encode("ascii") + contents[end:]


def release():
    if git("status", "--porcelain"):
        raise ValueError("Commit or stash your changes before running the release task.")

    branch = git("branch", "--show-current").strip()
    if not branch:
        raise ValueError("Check out the branch you want to release before running this task.")

    manifest = MANIFEST.read_bytes()
    lockfile = LOCKFILE.read_bytes()
    manifest_version = match_version(MANIFEST_VERSION, manifest, MANIFEST.name)
    lock_version = match_version(LOCK_VERSION, lockfile, LOCKFILE.name)
    if manifest_version[1] != lock_version[1]:
        raise ValueError("Cargo.toml and Cargo.lock have different TF2Monitor versions.")

    current = manifest_version[1].decode("ascii")
    resume = git("log", "-1", "--format=%s").strip() == f"Release v{current}"
    major, minor, patch = map(int, current.split("."))
    version = current if resume else f"{major}.{minor}.{patch + 1}"
    tag = f"v{version}"
    tag_ref = f"refs/tags/{tag}"
    head = git("rev-parse", "HEAD").strip()

    remote_tag = git("ls-remote", "--refs", "origin", tag_ref).strip()
    if remote_tag:
        if resume and remote_tag.split()[0] == head:
            print(f"{tag} is already pushed. Check the Rust (Windows) run in GitHub Actions.")
            return
        raise ValueError(f"{tag} already exists on origin. Sync the release branch before retrying.")

    if not resume:
        updates = [
            (MANIFEST, manifest, replace_version(manifest, manifest_version, version)),
            (LOCKFILE, lockfile, replace_version(lockfile, lock_version, version)),
        ]
        try:
            for path, _, updated in updates:
                path.write_bytes(updated)
            git(
                "commit", "--only", "-m", f"Release {tag}", "--", MANIFEST.name,
                LOCKFILE.name, capture=False
            )
        except (OSError, subprocess.CalledProcessError):
            # Restore only edits that still match what this task wrote.
            for path, original, updated in updates:
                if path.read_bytes() == updated:
                    path.write_bytes(original)
            raise
        head = git("rev-parse", "HEAD").strip()

    try:
        git(
            "push", "--atomic", "origin", f"{head}:refs/heads/{branch}",
            f"{head}:{tag_ref}", capture=False
        )
    except subprocess.CalledProcessError:
        raise ValueError(
            f"Push failed. The release commit for {tag} is local. "
            "Resolve the push error and rerun the task to use the same version."
        ) from None

    print(f"Pushed {tag}. GitHub Actions will test, build and publish the Windows release.")


if __name__ == "__main__":
    try:
        release()
    except subprocess.CalledProcessError as error:
        raise SystemExit((error.stderr or str(error)).strip()) from None
    except (OSError, ValueError) as error:
        raise SystemExit(str(error)) from None
