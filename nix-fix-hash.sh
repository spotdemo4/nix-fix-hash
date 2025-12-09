#!/usr/bin/env bash
# export PATH="${PATH}" placeholder

set -o errexit
set -o nounset
set -o pipefail

IFS=' ' read -r -a ARG <<< "${ARG_ENV:-}"
ARGS=( "${@}" "${ARG[@]}" )

function print() {
    echo "$1" >&2
}

function build() {
    local store
    local build_output
    local build_code

    if [[ -n "${DOCKER:-}" ]]; then
        print "running (docker): nix build ${ARGS[*]}"

        git config --global --add safe.directory "$(pwd)"

        build_output=$(
            nix build \
                --extra-experimental-features "nix-command flakes" \
                --accept-flake-config \
                --no-link \
                --no-warn-dirty \
                "${ARGS[@]}" 2>&1
        )
        build_code=$?

    else
        print "running: nix build ${ARGS[*]}"

        # use chroot store if not in container
        store=$(mktemp -d)

        build_output=$(
            nix build \
                --extra-experimental-features "nix-command flakes" \
                --accept-flake-config \
                --no-link \
                --no-warn-dirty \
                --store "${store}" \
                "${ARGS[@]}" 2>&1
        )
        build_code=$?

        # cleanup
        chmod -R u+w "${store}"
        rm -rf "${store}"
    fi

    if [[ ${build_code} -eq 0 ]]; then
        print "build successful"
        exit 0
    elif [[ ! "${build_output}" =~ "hash mismatch" ]]; then
        print "build failed, but no hash mismatch found"
        print "${build_output}"
        exit 1
    fi

    echo "${build_output}"
}

function extract_old_hash() {
    local build_output="$1"
    local old_hash=""

    if [[ ${build_output} =~ specified:[[:space:]]+([^[:space:]]+) ]]; then
        old_hash="${BASH_REMATCH[1]}"
    else
        print "could not extract old hash"
        exit 1
    fi

    if [[ -z "${old_hash}" ]]; then
        print "no old hash found"
        exit 1
    fi

    print "old hash: ${old_hash}"
    echo "${old_hash}"
}

function extract_new_hash() {
    local build_output="$1"
    local new_hash=""

    if [[ ${build_output} =~ got:[[:space:]]+([^[:space:]]+) ]]; then
        new_hash="${BASH_REMATCH[1]}"
    else
        print "could not extract new hash"
        exit 1
    fi

    if [[ -z "${new_hash}" ]]; then
        print "no new hash found"
        exit 1
    fi

    print "new hash: ${new_hash}"
    echo "${new_hash}"
}

function find_nix_files() {
    local old_hash="$1"
    local nix_files=()
    local files_with_hash=()

    readarray -d '' nix_files < <(find "$(pwd)" -type f -name "*.nix" -print0)
    if [[ ${#nix_files[@]} -eq 0 ]]; then
        print "no nix files found"
        exit 1
    fi

    for file in "${nix_files[@]}"; do
        if grep -q "${old_hash}" "${file}"; then
            files_with_hash+=("${file}")
        fi
    done
    if [[ ${#files_with_hash[@]} -eq 0 ]]; then
        print "no nix files found containing the old hash"
        exit 1
    fi

    printf "%s\n" "${files_with_hash[@]}"
}

function replace_hash() {
    local file="$1"
    local old_hash="$2"
    local new_hash="$3"

    if ! sd -F "${old_hash}" "${new_hash}" "${file}"; then
        print "failed to update hash in ${file}"
        exit 1
    else
        print "updated hash in ${file}"
    fi
}

while true; do
    build_output=$(build "$@")
    if [[ -z "${build_output}" ]]; then
        print "all hashes are up to date"
        exit 0
    fi

    old_hash=$(extract_old_hash "${build_output}")
    new_hash=$(extract_new_hash "${build_output}")
    readarray -t files < <(find_nix_files "${old_hash}")

    for file in "${files[@]}"; do
        replace_hash "${file}" "${old_hash}" "${new_hash}"
    done

    print ""
done
