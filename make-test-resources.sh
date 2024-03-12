#!/usr/bin/env bash

FAKE_ID_LONG=c0ffeec0ffeec0ffeec0ffeec0ffeec0ffeec0ffeec0ffeec0ffeec0ffeec0ff
FAKE_ID_SHORT=c0ffeec0ffee
FAKE_DT="2024-01-01T00:00:00.000000000Z"
FAKE_MAC="x6:de:ad:c0:ff:ee"

# shellcheck disable=SC2016
JQ_FILTERS=(
        '.Id = $fakeidlong'
        '.Config.Hostname = $fakeidshort'
        '.State.Pid = 1337'
        '(.Created, .State.StartedAt) = $fakedt'
        '(.ResolvConfPath, .HostnamePath, .HostsPath, .LogPath) |= (sub("[0-9a-f]{64}";$fakeidlong;"g"))'
        '(.GraphDriver.Data.LowerDir, .GraphDriver.Data.MergedDir, .GraphDriver.Data.UpperDir, .GraphDriver.Data.WorkDir) |= (sub("\/[0-9a-f]{64}(?<init>-init)?\/";"/X\(if .init then .init else "" end)/";"g"))'
        '(.NetworkSettings.SandboxID, .NetworkSettings.EndpointID, (.NetworkSettings.Networks[]|.NetworkID), (.NetworkSettings.Networks[]|.EndpointID)) |= (sub("[0-9a-f]{64}";"X"))'
        '.NetworkSettings.SandboxKey |= (sub("\/[0-9a-f]{12}";"/X"))'
        '(.NetworkSettings.Networks[]|.MacAddress) = $fakemac'
        '(.NetworkSettings.MacAddress|select(. == null or . == "")) = $fakemac'
        '.HostConfig.NetworkMode |= (sub(":[0-9a-f]{64}";":" + $fakeidlong))'
)

stop_and_rm_container () {
        local CONTAINER_ID="$1"
        docker stop -t 0 "${CONTAINER_ID}" > /dev/null 2>&1
        docker rm        "${CONTAINER_ID}" > /dev/null 2>&1
}

join_by_char() {
        local IFS="$1"
        shift
        echo "$*"
}

JQ_FILTER=$(join_by_char "|" "${JQ_FILTERS[@]}")

#
# Get version info
#

VERSION_JSON=$(curl -s --unix-socket /var/run/docker.sock --proto '=http' './version')
VERSION_API_VERSION=$(echo "$VERSION_JSON" | jq -r .ApiVersion)
mkdir -p "tests/resources/v${VERSION_API_VERSION}/"
echo "${VERSION_JSON}" | jq -r > "tests/resources/v${VERSION_API_VERSION}/version.body"

#
# Get sunny-default-bridge
#

stop_and_rm_container "sunny-default-bridge"
CONTAINER_ID=$(docker run --name sunny-default-bridge -d hashicorp/http-echo:1.0.0 -listen=:80 -text="✅ it works!")
INSPECT_JSON=$(curl -s --unix-socket /var/run/docker.sock --proto '=http' "./v${VERSION_API_VERSION}/containers/sunny-default-bridge/json")
mkdir -p "tests/resources/v${VERSION_API_VERSION}/containers/sunny-default-bridge"
echo "${INSPECT_JSON}" | \
        jq \
        --arg fakeidlong "$FAKE_ID_LONG" \
        --arg fakeidshort "$FAKE_ID_SHORT" \
        --arg fakedt "$FAKE_DT" \
        --arg fakemac "$FAKE_MAC" \
        "${JQ_FILTER}" \
        > "tests/resources/v${VERSION_API_VERSION}/containers/sunny-default-bridge/json.body"
stop_and_rm_container "sunny-default-bridge"

#
# Get sunny-default-bridge-container-subdomains-allowed
#

stop_and_rm_container "sunny-default-bridge-container-subdomains-allowed"
CONTAINER_ID=$(docker run --name sunny-default-bridge-container-subdomains-allowed -d --label=".com.github.petski.nss-docker-ng.container-subdomains-allowed=true" hashicorp/http-echo:1.0.0 -listen=:80 -text="✅ it works!")
INSPECT_JSON=$(curl -s --unix-socket /var/run/docker.sock --proto '=http' "./v${VERSION_API_VERSION}/containers/sunny-default-bridge-container-subdomains-allowed/json")
mkdir -p "tests/resources/v${VERSION_API_VERSION}/containers/sunny-default-bridge-container-subdomains-allowed"
echo "${INSPECT_JSON}" | \
        jq \
        --arg fakeidlong "$FAKE_ID_LONG" \
        --arg fakeidshort "$FAKE_ID_SHORT" \
        --arg fakedt "$FAKE_DT" \
        --arg fakemac "$FAKE_MAC" \
        "${JQ_FILTER}" \
        > "tests/resources/v${VERSION_API_VERSION}/containers/sunny-default-bridge-container-subdomains-allowed/json.body"
stop_and_rm_container "sunny-default-bridge-container-subdomains-allowed"

#
# Get rainy-404
#

stop_and_rm_container "rainy-404"
INSPECT_JSON=$(curl -s --unix-socket /var/run/docker.sock --proto '=http' "./v${VERSION_API_VERSION}/containers/rainy-404/json")
mkdir -p "tests/resources/v${VERSION_API_VERSION}/containers/rainy-404"
echo "${INSPECT_JSON}" | \
        jq \
        > "tests/resources/v${VERSION_API_VERSION}/containers/rainy-404/json.body"

#
# Get rainy-no-name
#

mkdir -p "tests/resources/v${VERSION_API_VERSION}/containers/rainy-no-name"
jq 'del(.Name)' \
        < "tests/resources/v${VERSION_API_VERSION}/containers/sunny-default-bridge/json.body" \
        > "tests/resources/v${VERSION_API_VERSION}/containers/rainy-no-name/json.body"

#
# Get rainy-no-network-mode
#

mkdir -p "tests/resources/v${VERSION_API_VERSION}/containers/rainy-no-network-mode"
jq 'del(.HostConfig.NetworkMode)' \
        < "tests/resources/v${VERSION_API_VERSION}/containers/sunny-default-bridge/json.body" \
        > "tests/resources/v${VERSION_API_VERSION}/containers/rainy-no-network-mode/json.body"

#
# Get rainy-network-mode-none
#

stop_and_rm_container "rainy-network-mode-none"
CONTAINER_ID=$(docker run --name rainy-network-mode-none --network none -d hashicorp/http-echo:1.0.0 -listen=:80 -text="✅ it works!")
INSPECT_JSON=$(curl -s --unix-socket /var/run/docker.sock --proto '=http' "./v${VERSION_API_VERSION}/containers/rainy-network-mode-none/json")
mkdir -p "tests/resources/v${VERSION_API_VERSION}/containers/rainy-network-mode-none"
echo "${INSPECT_JSON}" | \
        jq \
        --arg fakeidlong "$FAKE_ID_LONG" \
        --arg fakeidshort "$FAKE_ID_SHORT" \
        --arg fakedt "$FAKE_DT" \
        --arg fakemac "$FAKE_MAC" \
        "${JQ_FILTER}" \
        > "tests/resources/v${VERSION_API_VERSION}/containers/rainy-network-mode-none/json.body"
stop_and_rm_container "rainy-network-mode-none"

#
# Get rainy-network-mode-host
#

stop_and_rm_container "rainy-network-mode-host"
CONTAINER_ID=$(docker run --name rainy-network-mode-host --network host -d hashicorp/http-echo:1.0.0 -text="✅ it works!")
INSPECT_JSON=$(curl -s --unix-socket /var/run/docker.sock --proto '=http' "./v${VERSION_API_VERSION}/containers/rainy-network-mode-host/json")
mkdir -p "tests/resources/v${VERSION_API_VERSION}/containers/rainy-network-mode-host"
echo "${INSPECT_JSON}" | \
        jq \
        --arg fakeidlong "$FAKE_ID_LONG" \
        --arg fakeidshort "$FAKE_ID_SHORT" \
        --arg fakedt "$FAKE_DT" \
        --arg fakemac "$FAKE_MAC" \
        "${JQ_FILTER}" \
        > "tests/resources/v${VERSION_API_VERSION}/containers/rainy-network-mode-host/json.body"
stop_and_rm_container "rainy-network-mode-host"

#
# Get rainy-network-mode-container
#

stop_and_rm_container "rainy-network-mode-container-helper"
stop_and_rm_container "rainy-network-mode-container"
CONTAINER_ID=$(docker run --name rainy-network-mode-container-helper -d hashicorp/http-echo:1.0.0 -listen=:81 -text="✅ it works!")
CONTAINER_ID=$(docker run --name rainy-network-mode-container --network container:rainy-network-mode-container-helper -d hashicorp/http-echo:1.0.0 -listen=:80 -text="✅ it works!")
INSPECT_JSON=$(curl -s --unix-socket /var/run/docker.sock --proto '=http' "./v${VERSION_API_VERSION}/containers/rainy-network-mode-container/json")
mkdir -p "tests/resources/v${VERSION_API_VERSION}/containers/rainy-network-mode-container"
echo "${INSPECT_JSON}" | \
        jq \
        --arg fakeidlong "$FAKE_ID_LONG" \
        --arg fakeidshort "$FAKE_ID_SHORT" \
        --arg fakedt "$FAKE_DT" \
        --arg fakemac "$FAKE_MAC" \
        "${JQ_FILTER}" \
        > "tests/resources/v${VERSION_API_VERSION}/containers/rainy-network-mode-container/json.body"
stop_and_rm_container "rainy-network-mode-container-helper"
stop_and_rm_container "rainy-network-mode-container"

#
# Get rainy-zero-networks
#

mkdir -p "tests/resources/v${VERSION_API_VERSION}/containers/rainy-zero-networks"
jq '.NetworkSettings.Networks = {}' \
        < "tests/resources/v${VERSION_API_VERSION}/containers/sunny-default-bridge/json.body" \
        > "tests/resources/v${VERSION_API_VERSION}/containers/rainy-zero-networks/json.body"

#
# Get rainy-no-networks
#

mkdir -p "tests/resources/v${VERSION_API_VERSION}/containers/rainy-no-networks"
jq 'del(.NetworkSettings.Networks)' \
        < "tests/resources/v${VERSION_API_VERSION}/containers/sunny-default-bridge/json.body" \
        > "tests/resources/v${VERSION_API_VERSION}/containers/rainy-no-networks/json.body"

#
# Get rainy-network-not-exists
#

mkdir -p "tests/resources/v${VERSION_API_VERSION}/containers/rainy-network-not-exists"
jq '.NetworkSettings.Networks.foo = .NetworkSettings.Networks.bridge | del(.NetworkSettings.Networks.bridge) | .HostConfig.NetworkMode = "bar"' \
        < "tests/resources/v${VERSION_API_VERSION}/containers/sunny-default-bridge/json.body" \
        > "tests/resources/v${VERSION_API_VERSION}/containers/rainy-network-not-exists/json.body"

#
# Get rainy-ip-address-empty
#

mkdir -p "tests/resources/v${VERSION_API_VERSION}/containers/rainy-ip-address-empty"
jq '.NetworkSettings.Networks.bridge.IPAddress = ""' \
        < "tests/resources/v${VERSION_API_VERSION}/containers/sunny-default-bridge/json.body" \
        > "tests/resources/v${VERSION_API_VERSION}/containers/rainy-ip-address-empty/json.body"

#
# Get rainy-no-ip-address
#

mkdir -p "tests/resources/v${VERSION_API_VERSION}/containers/rainy-no-ip-address"
jq 'del(.NetworkSettings.Networks.bridge.IPAddress)' \
        < "tests/resources/v${VERSION_API_VERSION}/containers/sunny-default-bridge/json.body" \
        > "tests/resources/v${VERSION_API_VERSION}/containers/rainy-no-ip-address/json.body"

#
# Get rainy-unparseable-ip-address
#

mkdir -p "tests/resources/v${VERSION_API_VERSION}/containers/rainy-unparseable-ip-address"
jq '.NetworkSettings.Networks.bridge.IPAddress = "x.x.x.x"' \
        < "tests/resources/v${VERSION_API_VERSION}/containers/sunny-default-bridge/json.body" \
        > "tests/resources/v${VERSION_API_VERSION}/containers/rainy-unparseable-ip-address/json.body"

#
# Get rainy-no-id
#

mkdir -p "tests/resources/v${VERSION_API_VERSION}/containers/rainy-no-id"
jq 'del(.Id)' \
        < "tests/resources/v${VERSION_API_VERSION}/containers/sunny-default-bridge/json.body" \
        > "tests/resources/v${VERSION_API_VERSION}/containers/rainy-no-id/json.body"
