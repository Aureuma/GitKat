# Changelog

## 0.4.0

- Renamed the project and distribution to GitKat.
- Renamed the Python package namespace to `gitkat`.
- Updated repo links, badges, docs branding, and build config for the new name.
- Added PyPI release workflow with OIDC trusted publishing.
- Added GitHub and PyPI buttons plus a logo asset for docs/README.
- Added Codecov token support, reporting, and coverage XML uploads in CI.
- Added dependency groups for uv and updated CI to create a venv before install.
- Added CI workflow for linting, tests, and docs builds.
- Added build status and coverage badges to the README.
- Updated developer install instructions to use uv dependency groups.

## 0.3.2

- Fixed newline escaping in rewrite callbacks for git-filter-repo.

## 0.3.1

- Fixed git-filter-repo callback indentation for rewrite mappings.

## 0.3.0

- Renamed the project and distribution to GitKat.
- Renamed the Python package namespace to `gitkat`.
- Updated repo links, badges, docs branding, and build config for the new name.

## 0.2.11

- Fixed PyPI trusted publishing workflow to match OIDC configuration.

## 0.2.10

- Added PyPI release workflow with OIDC trusted publishing.
- Updated branding, docs assets, and repo links for the Aureuma org.
- Added GitHub and PyPI buttons plus a logo asset for docs/README.

## 0.2.9

- Relaxed Codecov upload failure to prevent CI failures without token configuration.
- Added Codecov token support in CI.

## 0.2.8

- Added dependency groups for uv and updated CI to create a venv before install.
- Added Codecov reporting and badge for live coverage status.
- Added Codecov configuration and coverage XML upload.
- Updated developer install instructions to use uv dependency groups.

## 0.2.7

- Fixed lint issues and ensured CI checks pass cleanly.

## 0.2.6

- Added CI workflow for linting, tests, and docs builds.
- Added build status badge to the README.

## 0.2.5

- Added coverage badge and documented coverage command in the README.

## 0.2.4

- Fixed CLI syntax and rewrite error handling.
- Expanded test suite to reach near-full coverage.

## 0.2.3

- Added project URLs for PyPI metadata and committed uv.lock for reproducible installs.
- Fixed uv dev install examples for shells that treat extras as globs.

## 0.2.2

- Added MkDocs documentation and project metadata for publishing.
- Added collaboration files and roadmap.

## 0.2.1

- Ported legacy shell commands to Python modules and added the `gk` CLI.
- Implemented rewrite behavior with git-filter-repo callbacks.
- Added unit tests for core commands and GitHub email logic.

## 0.2.0

- Introduced Python package scaffold and uv-based project layout.
