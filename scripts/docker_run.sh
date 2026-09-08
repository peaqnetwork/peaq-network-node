#!/usr/bin/env bash
# This script is meant to be run on Unix/Linux based systems
set -e

echo "*** Start PEAQ Network Node ***"

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
compose_file="${repository_root}/scripts/docker-compose.yml"

docker-compose --project-directory "${repository_root}" -f "${compose_file}" down --remove-orphans
docker-compose --project-directory "${repository_root}" -f "${compose_file}" run --rm --service-ports dev "$@"
