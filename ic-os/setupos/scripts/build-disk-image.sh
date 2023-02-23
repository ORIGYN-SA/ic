#!/usr/bin/env bash

# Build bootable SetupOS disk image

set -o errexit
set -o pipefail
# NOTE: Validating inputs manually
#set -o nounset

SHELL="/bin/bash"
PATH="/sbin:/bin:/usr/sbin:/usr/bin"

BASE_DIR="$(dirname "${BASH_SOURCE[0]}")/.."
TMP_DIR="$(mktemp -d)"
trap "rm -rf ${TMP_DIR}" EXIT
TOOL_DIR="${BASE_DIR}/../../toolchains/sysimage/"

# Fixed timestamp for reproducible build
TOUCH_TIMESTAMP="200901031815.05"
export SOURCE_DATE_EPOCH="1231006505"

function usage() {
    cat <<EOF

Usage:
  build-disk-image -o outfile -v version -t dev] [-p password]

  Build whole disk of IC SetupOS image.

  -d deployment: Define the deployment name
                 Default: mainnet
  -f host-os-img: Specify the HostOS disk-image file; mandatory
                  Default: ./host-os.img.tar.gz
  -g guest-os-img: Specify the GuestOS disk-image file; mandatory
                   Default: ./guest-os.img.tar.gz
  -k nns-public-key: Specify the NNS public key
                     Example: ./nns_public_key.pem
  -l logging-hosts: Define the logging hosts/destination
                    Default: telemetry01.mainnet.dfinity.network
                             telemetry02.mainnet.dfinity.network
                             telemetry03.mainnet.dfinity.network
  -m memory: Set the amount of memory in GiB (Gibibytes) for the GuestOS.
             Default: 490
  -n nns-url: Define the NNS URL for the GuestOS.
              Default: https://nns.ic0.app
  -o outfile: Name of output file; mandatory
  -p password: Set root password for console access. This is only allowed
               for "dev" images
  -s nameserver: Define the DNS name servers.
                 Default: 2606:4700:4700::1111 2606:4700:4700::1001
                          2001:4860:4860::8888 2001:4860:4860::8844
  -t image-type: The type of image to build. Must be either "dev" or "prod".
                 If nothing is specified, defaults to building "prod" image.
  -v version: The version written into the image; mandatory

EOF
}

BUILD_TYPE=prod
while getopts "d:f:g:k:l:m:n:o:p:s:t:v:" OPT; do
    case "${OPT}" in
        d)
            DEPLOYMENT="${OPTARG}"
            ;;
        f)
            HOST_OS="${OPTARG}"
            ;;
        g)
            GUEST_OS="${OPTARG}"
            ;;
        k)
            KEY="${OPTARG}"
            ;;
        l)
            LOGGING="${OPTARG}"
            ;;
        m)
            MEMORY="${OPTARG}"
            ;;
        n)
            NNS_URL="${OPTARG}"
            ;;
        o)
            OUT_FILE="${OPTARG}"
            ;;
        p)
            ROOT_PASSWORD="${OPTARG}"
            ;;
        s)
            NAME_SERVERS="${OPTARG}"
            ;;
        t)
            BUILD_TYPE="${OPTARG}"
            ;;
        v)
            VERSION="${OPTARG}"
            ;;
        *)
            usage >&2
            exit 1
            ;;
    esac
done

# Set arguments if undefined
DEPLOYMENT="${DEPLOYMENT:=mainnet}"
HOST_OS="${HOST_OS:=${BASE_DIR}/host-os.img.tar.gz}"
GUEST_OS="${GUEST_OS:=${BASE_DIR}/guest-os.img.tar.gz}"
LOGGING="${LOGGING:=elasticsearch-node-0.mercury.dfinity.systems:443 elasticsearch-node-1.mercury.dfinity.systems:443 elasticsearch-node-2.mercury.dfinity.systems:443 elasticsearch-node-3.mercury.dfinity.systems:443}"
MEMORY="${MEMORY:=490}"
NNS_URL="${NNS_URL:=https://nns.ic0.app}"
NAME_SERVERS="${NAME_SERVERS:=2606:4700:4700::1111 2606:4700:4700::1001 2001:4860:4860::8888 2001:4860:4860::8844}"
BASE_IMAGE_FILE="${BASE_DIR}/rootfs/docker-base.${BUILD_TYPE}"

if [ "${OUT_FILE}" == "" ]; then
    usage >&2
    exit 1
fi

if [ "${BUILD_TYPE}" != "dev" -a "${BUILD_TYPE}" != "prod" ]; then
    echo "Unknown build type: ${BUILD_TYPE}" >&2
    exit 1
fi

if [ "${ROOT_PASSWORD}" != "" -a "${BUILD_TYPE}" != "dev" ]; then
    echo "Root password is valid only for build type 'dev'" >&2
    exit 1
fi

if [ "${VERSION}" == "" ]; then
    echo "Version needs to be specified for build to succeed" >&2
    usage >&2
    exit 1
fi

function log_and_exit_on_error() {
    local exit_code="${1}"
    local log_message="${2}"

    if [ "${exit_code}" -ne 0 ]; then
        echo "${log_message}" >&2
        exit "${exit_code}"
    fi
}

function log_start() {
    TIME_START=$(date '+%s')

    echo "SetupOS Builder - Start"
    log_and_exit_on_error "${?}" "Unable to start SetupOS builder."
}

function validate_guest_os() {
    echo "* Validating GuestOS disk-image..."
    if [ ! -r "${GUEST_OS}" ]; then
        log_and_exit_on_error "1" "Unable to find or read GuestOS disk-image."
    fi
}

function validate_host_os() {
    echo "* Validating HostOS disk-image..."
    if [ ! -r "${HOST_OS}" ]; then
        log_and_exit_on_error "1" "Unable to find or read HostOS disk-image."
    fi
}

function prepare_config_partition() {
    echo "* Preparing config partition..."

    CONFIG_TMP="${TMP_DIR}/config"
    mkdir -p ${CONFIG_TMP}

    (
        cat <<EOF
# Please update the template/example below.
#
# If you need help, please do not hesitate to contact the
# Internet Computer Association.
#
ipv6_prefix=2a00:fb01:400:100
ipv6_subnet=/64
ipv6_gateway=2a00:fb01:400:100::1
EOF
    ) >"${CONFIG_TMP}/config.ini"

    mkdir -p ${CONFIG_TMP}/ssh_authorized_keys

    (
        cat <<EOF
# Please insert your SSH public keys here.
#
# Each line of the file must only contain one key. For details on the format,
# please consult 'man authorized_keys'.
#
EOF
    ) >"${CONFIG_TMP}/ssh_authorized_keys/admin"

    # Fix timestamps for reproducible build
    touch -t ${TOUCH_TIMESTAMP} \
        ${CONFIG_TMP}/config.ini \
        ${CONFIG_TMP}/ssh_authorized_keys \
        ${CONFIG_TMP}/ssh_authorized_keys/admin

    cd ${TMP_DIR}
    # tar flags set for build determinism
    tar -cvf "${TMP_DIR}/config.tar" --sort=name --owner=root:0 --group=root:0 "--mtime=UTC 1970-01-01 00:00:00" -C "${TMP_DIR}/" config/
    cd -
}

function prepare_data_partition() {
    echo "* Preparing data partition..."

    DATA_TMP="${TMP_DIR}/data"
    mkdir -p ${DATA_TMP}

    (
        cat <<EOF
{
  "deployment": {
    "name": "{{ deployment_name }}"
  },
  "logging": {
    "hosts": "{{ logging_hosts }}"
  },
  "nns": {
    "url": "{{ nns_url }}"
  },
  "dns": {
    "name_servers": "{{ dns_name_servers }}"
  },
  "resources": {
    "memory": "{{ resources_memory }}"
  }
}
EOF
    ) >"${DATA_TMP}/deployment.json"

    if [ ! -z "${KEY}" ]; then
        cat ${KEY} >"${DATA_TMP}/nns_public_key.pem"
    else
        (
            cat <<EOF
-----BEGIN PUBLIC KEY-----
MIGCMB0GDSsGAQQBgtx8BQMBAgEGDCsGAQQBgtx8BQMCAQNhAIFMDm7HH6tYOwi9
gTc8JVw8NxsuhIY8mKTx4It0I10U+12cDNVG2WhfkToMCyzFNBWDv0tDkuRn25bW
W5u0y3FxEvhHLg1aTRRQX/10hLASkQkcX4e5iINGP5gJGguqrg==
-----END PUBLIC KEY-----
EOF
        ) >"${DATA_TMP}/nns_public_key.pem"
    fi

    # Inject deployment configuration
    sed -i "s@{{ nns_url }}@${NNS_URL}@g" "${DATA_TMP}/deployment.json"
    sed -i "s@{{ deployment_name }}@${DEPLOYMENT}@g" "${DATA_TMP}/deployment.json"
    sed -i "s@{{ logging_hosts }}@${LOGGING}@g" "${DATA_TMP}/deployment.json"
    sed -i "s@{{ dns_name_servers }}@${NAME_SERVERS}@g" "${DATA_TMP}/deployment.json"
    sed -i "s@{{ resources_memory }}@${MEMORY}@g" "${DATA_TMP}/deployment.json"

    # Copy disk-image files
    cp --preserve=timestamp "${GUEST_OS}" "${DATA_TMP}/guest-os.img.tar.gz"
    cp --preserve=timestamp "${HOST_OS}" "${DATA_TMP}/host-os.img.tar.gz"

    # Fix timestamps for reproducible build
    touch -t ${TOUCH_TIMESTAMP} \
        ${DATA_TMP}/deployment.json \
        ${DATA_TMP}/nns_public_key.pem \
        ${DATA_TMP}/guest-os.img.tar.gz \
        ${DATA_TMP}/host-os.img.tar.gz

    #tar -cvf "${TMP_DIR}/data.tar" -C "${TMP_DIR}/" data/
    cd ${TMP_DIR}
    tar -cvf "${TMP_DIR}/data.tar" -C "${TMP_DIR}/" data/
    cd -
}

function assemble_and_populate_image() {
    echo "${VERSION}" >"${TMP_DIR}/version.txt"
    touch -t ${TOUCH_TIMESTAMP} ${TMP_DIR}/version.txt

    "${TOOL_DIR}"/docker_tar.py -o "${TMP_DIR}/boot-tree.tar" "${BASE_DIR}/bootloader"
    "${TOOL_DIR}"/docker_tar.py -o "${TMP_DIR}/rootfs-tree.tar" --build-arg ROOT_PASSWORD="${ROOT_PASSWORD}" --file-build-arg BASE_IMAGE="${BASE_IMAGE_FILE}" "${BASE_DIR}/rootfs"

    "${TOOL_DIR}"/build_vfat_image.py -o "${TMP_DIR}/partition-esp.tar" -s 50M -p boot/efi -i "${TMP_DIR}/boot-tree.tar"
    "${TOOL_DIR}"/build_vfat_image.py -o "${TMP_DIR}/partition-grub.tar" -s 50M -p boot/grub -i "${TMP_DIR}/boot-tree.tar" \
        "${BASE_DIR}/bootloader/grub.cfg:/boot/grub/grub.cfg:644" \
        "${BASE_DIR}/bootloader/grubenv:/boot/grub/grubenv:644"

    "${TOOL_DIR}"/build_fat32_image.py -o "${TMP_DIR}/partition-config.tar" -s 50M -p config/ -l CONFIG -i "${TMP_DIR}/config.tar"
    "${TOOL_DIR}"/build_ext4_image.py -o "${TMP_DIR}/partition-data.tar" -s 1750M -p data/ -i "${TMP_DIR}/data.tar"

    tar xOf "${TMP_DIR}"/rootfs-tree.tar --occurrence=1 etc/selinux/default/contexts/files/file_contexts >"${TMP_DIR}/file_contexts"

    "${TOOL_DIR}"/build_ext4_image.py -o "${TMP_DIR}/partition-boot.tar" -s 100M -i "${TMP_DIR}/rootfs-tree.tar" -S "${TMP_DIR}/file_contexts" -p boot/ \
        "${TMP_DIR}/version.txt:/boot/version.txt:0644" \
        "${BASE_DIR}/bootloader/extra_boot_args:/boot/extra_boot_args:0644"

    "${TOOL_DIR}"/build_ext4_image.py --strip-paths /run /boot -o "${TMP_DIR}/partition-root.tar" -s 1750M -i "${TMP_DIR}/rootfs-tree.tar" -S "${TMP_DIR}/file_contexts" \
        "${TMP_DIR}/version.txt:/opt/ic/share/version.txt:0644"

    "${TOOL_DIR}"/build_disk_image.py -o "${TMP_DIR}/disk.img.tar" -p "${BASE_DIR}/scripts/partitions.csv" \
        ${TMP_DIR}/partition-esp.tar \
        ${TMP_DIR}/partition-grub.tar \
        ${TMP_DIR}/partition-config.tar \
        ${TMP_DIR}/partition-data.tar \
        ${TMP_DIR}/partition-boot.tar \
        ${TMP_DIR}/partition-root.tar
}

function provide_raw_image() {
    # For compatibility with previous use of this script, provide the raw
    # image as output from this program.
    OUT_DIRNAME="$(dirname "${OUT_FILE}")"
    OUT_BASENAME="$(basename "${OUT_FILE}")"
    tar xf "${TMP_DIR}/disk.img.tar" --transform="s/disk.img/${OUT_BASENAME}/" -C "${OUT_DIRNAME}"
    # increase size a bit, for immediate qemu use (legacy)
    truncate --size 4G "${OUT_FILE}"
}

function log_end() {
    local time_end=$(date '+%s')
    local time_exec=$(expr "${time_end}" - "${TIME_START}")
    local time_hr=$(date -d "1970-01-01 ${time_exec} sec" '+%H:%M:%S')

    echo "SetupOS Builder - End (${time_hr})"
    log_and_exit_on_error "${?}" "Unable to end SetupOS builder."
}

# Establish run order
function main() {
    log_start
    validate_guest_os
    validate_host_os
    prepare_config_partition
    prepare_data_partition
    assemble_and_populate_image
    provide_raw_image
    log_end
}

main
