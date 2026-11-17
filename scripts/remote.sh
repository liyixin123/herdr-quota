#!/bin/sh
# Download a verified native manager; never modify the installed copy in shell.
set +x
set -eu
umask 077
fail() { printf '%s\n' "herdr-quota: $* Fix the problem and rerun the same command." >&2; exit 1; }
action=${1:-install}
case "$action" in install|uninstall) [ "$#" -eq 0 ] || shift ;; *) fail 'Usage: remote.sh install|uninstall [native options].' ;; esac
for tool in curl tar uname mktemp awk chmod rm; do
    command -v "$tool" >/dev/null 2>&1 || fail "Missing $tool; install it first."
done
case "$(uname -s)/$(uname -m)" in
    Darwin/arm64|Darwin/aarch64) target=aarch64-apple-darwin ;;
    Linux/x86_64|Linux/amd64) target=x86_64-unknown-linux-musl ;;
    *) fail 'Unsupported platform; supported: macOS ARM64 and Linux x86_64.' ;;
esac
if command -v sha256sum >/dev/null 2>&1; then
    sha() { sha256sum "$1"; }
elif command -v shasum >/dev/null 2>&1; then
    sha() { shasum -a 256 "$1"; }
else
    fail 'Missing SHA256 tool; install sha256sum or shasum.'
fi
stage=$(mktemp -d "${TMPDIR:-/tmp}/herdr-quota.XXXXXXXX") || fail 'Cannot create private staging directory.'
trap 'rm -rf "$stage"' 0
trap 'exit 130' INT
trap 'exit 143' TERM
trap 'exit 129' HUP
repo=liyixin123/herdr-quota
api=https://api.github.com/repos/$repo
web=https://github.com/$repo/releases
token=${GH_TOKEN:-${GITHUB_TOKEN:-}}
# Credentials never appear in curl argv; RFC 6750 bearer-token alphabet prevents header/config injection.
if [ -n "$token" ]; then
    case "$token" in *[!a-zA-Z0-9._~+/=-]*) fail 'Invalid GitHub token format; use GH_TOKEN or GITHUB_TOKEN.' ;; esac
    command -v jq >/dev/null 2>&1 || fail 'Private downloads require jq; install jq and rerun with GH_TOKEN or GITHUB_TOKEN.'
    printf 'header = "Authorization: Bearer %s"\n' "$token" > "$stage/auth"
    unset token
    api_get() {
        # No -L: redirect is followed by a fresh credential-free request.
        code=$(curl -q -K "$stage/auth" --silent --show-error --fail --proto '=https' --connect-timeout 15 --max-time 120 -H "Accept: $2" -D "$stage/headers" -o "$3" -w '%{http_code}' "$1" 2>/dev/null) || fail 'GitHub API download failed; check network, token Contents read access, and published release.'
        case "$code" in
            200) ;;
            302)
                [ "$2" = application/octet-stream ] || fail 'Unexpected GitHub metadata redirect.'
                location=$(awk 'tolower($1)=="location:" {sub(/^[^:]*:[ \t]*/, ""); sub(/\r$/, ""); print; n++} END {if(n!=1) exit 1}' "$stage/headers") || fail 'Invalid asset redirect.'
                case "$location" in https://release-assets.githubusercontent.com/*|https://objects.githubusercontent.com/*) ;; *) fail 'Unsafe asset redirect host or protocol.' ;; esac
                download "$location" "$3"
                ;;
            *) fail 'Unexpected GitHub API status; check authentication and release availability.' ;;
        esac
    }
fi
download() {
    curl -q --silent --show-error --fail --location --proto '=https' --proto-redir '=https' --connect-timeout 15 --max-time 120 -o "$2" "$1" 2>/dev/null || fail 'Release download failed; check network and release availability (private repositories need GH_TOKEN or GITHUB_TOKEN).'
}
if [ -f "$stage/auth" ]; then
    # Optional tag pins post-publication acceptance to the newly published release.
    endpoint=latest
    if [ -n "${HERDR_QUOTA_TAG:-}" ]; then
        printf '%s\n' "$HERDR_QUOTA_TAG" | awk '/^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$/ {ok=1} END {exit !ok}' || fail 'Invalid HERDR_QUOTA_TAG.'
        endpoint=tags/$HERDR_QUOTA_TAG
    fi
    api_get "$api/releases/$endpoint" application/vnd.github+json "$stage/release.json"
    tag=$(jq -er '.tag_name | strings | select(test("^v[0-9]+\\.[0-9]+\\.[0-9]+(-[0-9A-Za-z.-]+)?$"))' "$stage/release.json") || fail 'Invalid GitHub release tag or JSON.'
    [ -z "${HERDR_QUOTA_TAG:-}" ] || [ "$tag" = "$HERDR_QUOTA_TAG" ] || fail 'Release tag mismatch.'
    archive=herdr-quota-$tag-$target.tar.gz
    asset_id() {
        jq -er --arg name "$1" '[.assets[] | select(.name == $name and .state == "uploaded")] | select(length == 1) | .[0].id | numbers | select(. > 0 and floor == .) | tostring | select(test("^[0-9]+$"))' "$stage/release.json" || fail 'Release assets missing or duplicated; check published release.'
    }
    sums_id=$(asset_id SHA256SUMS)
    archive_id=$(asset_id "$archive")
    api_get "$api/releases/assets/$sums_id" application/octet-stream "$stage/SHA256SUMS"
else
    if [ -n "${HERDR_QUOTA_TAG:-}" ]; then
        printf '%s\n' "$HERDR_QUOTA_TAG" | awk '/^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$/ {ok=1} END {exit !ok}' || fail 'Invalid HERDR_QUOTA_TAG.'
        download "$web/download/$HERDR_QUOTA_TAG/SHA256SUMS" "$stage/SHA256SUMS"
    else
        download "$web/latest/download/SHA256SUMS" "$stage/SHA256SUMS"
    fi
    # Derive the version from the manifest; never mix latest checksums with a later latest archive.
    archive=$(awk -v target="$target" 'length($1)==64 && $1 !~ /[^0-9a-f]/ && $2 ~ ("^herdr-quota-v[0-9]+\\.[0-9]+\\.[0-9]+(-[0-9A-Za-z.-]+)?-" target "\\.tar\\.gz$") && NF==2 {print $2; n++} END {if(n!=1) exit 1}' "$stage/SHA256SUMS") || fail 'Missing or duplicate platform checksum.'
    tag=${archive#herdr-quota-}
    tag=${tag%-$target.tar.gz}
    [ -z "${HERDR_QUOTA_TAG:-}" ] || [ "$tag" = "$HERDR_QUOTA_TAG" ] || fail 'Release tag mismatch.'
fi
expected=$(awk -v name="$archive" '$2==name {if(NF!=2 || length($1)!=64 || $1 ~ /[^0-9a-f]/) exit 1; print $1; n++} END {if(n!=1) exit 1}' "$stage/SHA256SUMS") || fail 'Missing, invalid, or duplicate archive checksum.'
if [ -f "$stage/auth" ]; then
    api_get "$api/releases/assets/$archive_id" application/octet-stream "$stage/package.tar.gz"
else
    download "$web/download/$tag/$archive" "$stage/package.tar.gz"
fi
actual=$(sha "$stage/package.tar.gz") || fail 'Cannot calculate archive SHA256.'
[ "${actual%% *}" = "$expected" ] || fail 'Archive SHA256 mismatch; installed program was preserved.'
# Require exactly one regular root file before extraction. No links, paths, or extra files.
LC_ALL=C tar -tzf "$stage/package.tar.gz" > "$stage/list" 2>/dev/null || fail 'Invalid release archive.'
awk '$0!="herdr-quota" {exit 1} END {if(NR!=1) exit 1}' "$stage/list" || fail 'Unsafe release archive; expected only root herdr-quota.'
LC_ALL=C tar -tvzf "$stage/package.tar.gz" > "$stage/details" 2>/dev/null || fail 'Invalid archive metadata.'
awk 'substr($0,1,1)!="-" {exit 1} END {if(NR!=1) exit 1}' "$stage/details" || fail 'Unsafe release archive; executable must be a regular file.'
tar -xzf "$stage/package.tar.gz" -C "$stage" 2>/dev/null || fail 'Cannot extract release archive.'
[ -f "$stage/herdr-quota" ] && [ ! -L "$stage/herdr-quota" ] || fail 'Missing release executable.'
chmod u+x "$stage/herdr-quota"
version=$("$stage/herdr-quota" --version) || fail 'Cannot execute release; check platform compatibility.'
[ "$version" = "herdr-quota ${tag#v}" ] || fail 'Release executable version mismatch.'
"$stage/herdr-quota" "$action" "$@"
