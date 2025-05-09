#!/bin/bash

command="${1,,}"
release="${2,,}"

release_flag=""
build_type="debug"
if [[ "$release" == "true" || "$release" == "1" || "$release" == "yes" ]]; then
    release_flag="--release"
    build_type="release"
fi

case "${command}" in
    "bin")
        cargo build ${release_flag} -p toi_server
        mv /usr/app/target/${build_type}/toi_server /usr/local/bin/toi_server
        ;;
    "cook")
        cargo chef cook ${release_flag} --recipe-path recipe.json
        ;;
    "test")
        cargo test --no-run ${release_flag}
        ;;
esac
