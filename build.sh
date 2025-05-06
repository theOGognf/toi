#!/bin/bash

command="${1,,}"
release="${2,,}"

case "${command}" in
    "build")
        if [[ "${release}" == "true" || "${release}" == "1" || "${release}" == "yes" ]]; then
            cargo build --release -p toi_server
            mv /usr/app/target/release/toi_server /usr/local/bin/toi_server
        else
            cargo build -p toi_server
            mv /usr/app/target/debug/toi_server /usr/local/bin/toi_server
        fi
        ;;
    "cook")
        if [[ "${release}" == "true" || "${release}" == "1" || "${release}" == "yes" ]]; then
            cargo chef cook --release --recipe-path recipe.json
        else
            cargo chef cook --recipe-path recipe.json
        fi
        ;;
esac
