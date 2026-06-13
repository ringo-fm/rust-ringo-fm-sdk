# Release new workspace version (tag + push)

set shell := ["bash", "-cu"]

packages := "ringo-fm-sys ringo-fm"

release-check:
    cargo test -p ringo-fm-sys -p ringo-fm --all-features
    cargo build -p ringo-fm-sys -p ringo-fm --release --all-features
    cargo publish -p ringo-fm-sys --dry-run
    cargo publish -p ringo-fm --dry-run

release: release-check
    version=$(cargo metadata --no-deps --format-version 1 | jq -r '.workspace_members[0] as $member | .packages[] | select(.id == $member) | .version'); \
    test -n "$version"; \
    test "$(git rev-parse --abbrev-ref HEAD)" = "main"; \
    test -z "$(git status --porcelain)"; \
    git tag "v${version}"; \
    git push origin "v${version}"

publish-local-dry-run:
    for package in {{packages}}; do cargo publish -p "$package" --dry-run; done
