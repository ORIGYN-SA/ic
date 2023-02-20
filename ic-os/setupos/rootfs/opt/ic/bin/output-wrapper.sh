#!/usr/bin/env bash

set -o nounset
set -o pipefail

SHELL="/bin/bash"
PATH="/sbin:/bin:/usr/sbin:/usr/bin"

SCRIPT="$1"
TEE_TARGET="$2"

${SCRIPT} 2>&1 | tee ${TEE_TARGET}
