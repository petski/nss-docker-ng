extern crate debug_print;
extern crate docker_api;

use debug_print::debug_eprintln;
use docker_api::Docker;
use libnss::host::{AddressFamily, Addresses, Host, HostHooks};
use libnss::interop::Response;
use libnss::libnss_host_hooks;
use std::error::Error;
use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;

#[cfg(test)]
use mocktopus::macros::mockable;

static SUFFIX: &str = ".docker";
static DOCKER_URI: &str = "unix:///var/run/docker.sock";
static CONTAINER_SUBDOMAINS_ALLOWED_LABEL: &str =
    ".com.github.petski.nss-docker-ng.container-subdomains-allowed";

struct DockerNG;
libnss_host_hooks!(docker_ng, DockerNG);

impl HostHooks for DockerNG {
    fn get_all_entries() -> Response<Vec<Host>> {
        Response::NotFound // TODO: Implement me, see https://github.com/petski/nss-docker-ng/issues/1
    }

    fn get_host_by_name(name: &str, family: AddressFamily) -> Response<Host> {
        match get_host_by_name(name, family) {
            Ok(Some(host)) => Response::Success(host),
            Ok(None) => Response::NotFound,
            Err(_e) => {
                debug_eprintln!("get_host_by_name '{}' failed: {}", name, _e);
                Response::Unavail
            }
        }
    }

    fn get_host_by_addr(_: IpAddr) -> Response<Host> {
        Response::NotFound // TODO: Implement me, see https://github.com/petski/nss-docker-ng/issues/2
    }
}

#[cfg_attr(test, mockable)]
pub fn get_docker_uri() -> String {
    DOCKER_URI.to_string()
}

#[tokio::main]
async fn get_host_by_name(
    query: &str,
    family: AddressFamily,
) -> Result<Option<Host>, Box<dyn Error>> {
    // Check if the query ends with the expected suffix and if the address family is IPv4
    if !(query.ends_with(SUFFIX) &&
        family == AddressFamily::IPv4 && // TODO you no v6? See https://github.com/petski/nss-docker-ng/issues/3
        query.len() > SUFFIX.len())
    {
        return Ok(None);
    }

    // Initialize Docker API client
    let mut docker = Docker::new(get_docker_uri())?;
    docker.adjust_api_version().await?;

    // Strip suffix from query
    let query_stripped = &query[..query.len() - SUFFIX.len()];

    // Fetch container information
    let inspect_result = 'block: {
        match docker.containers().get(query_stripped).inspect().await {
            Ok(query_stripped_result) => query_stripped_result,
            Err(_e) => {
                debug_eprintln!("Failed to inspect container '{}': {}", query_stripped, _e);

                if let Some((last_dot_index, _)) = query_stripped.match_indices('.').next_back() {
                    let query_stripped_main_domain =
                        &query_stripped[(last_dot_index + '.'.len_utf8())..];
                    match docker
                        .containers()
                        .get(query_stripped_main_domain)
                        .inspect()
                        .await
                    {
                        Ok(query_stripped_main_domain_result) => {
                            match query_stripped_main_domain_result.config.as_ref().and_then(
                                |config| {
                                    config.labels.as_ref().and_then(|labels| {
                                        labels.get(CONTAINER_SUBDOMAINS_ALLOWED_LABEL)
                                    })
                                },
                            ) {
                                Some(label_value) => {
                                    if label_value.eq("true")
                                        || label_value.eq("True")
                                        || label_value.eq("1")
                                    {
                                        break 'block query_stripped_main_domain_result;
                                    } else {
                                        return Ok(None);
                                    }
                                }
                                None => return Ok(None),
                            }
                        }
                        Err(_e) => return Ok(None),
                    }
                }
                return Ok(None);
            }
        }
    };

    // Name of the container (remove leading "/" if necessary)
    let name = match inspect_result.name {
        Some(mut name) => {
            if name.starts_with('/') {
                name.remove(0);
            }
            name
        }
        None => return Err("No Name".into()),
    };

    // From https://docs.docker.com/engine/api/v1.44/#tag/Container/operation/ContainerInspect:
    // > Network mode to use for this container. Supported standard values are: bridge, host, none, and
    // > container:<name|id>. Any other value is taken as a custom network's name to which this container
    // > should connect to
    let mut network_mode = match inspect_result.host_config.as_ref().and_then(|host_config| {
        host_config
            .get("NetworkMode")
            .and_then(|value| value.as_str())
    }) {
        Some(network_mode) => {
            debug_eprintln!("Container '{}' has NetworkMode '{}'", name, network_mode);
            network_mode
        }
        None => return Err("Could not find NetworkMode".into()),
    };

    // NetworkMode host, none, and those starting with "container:" don't have an IP address
    if ["none", "host"].contains(&network_mode) || network_mode.starts_with("container:") {
        debug_eprintln!(
            "Container '{}' is in NetworkMode {}, no IP here",
            name,
            network_mode
        );
        return Ok(None);
    }

    // Extract networks from network settings
    let networks = match inspect_result
        .network_settings
        .and_then(|settings| settings.networks)
    {
        Some(networks) => {
            if networks.is_empty() {
                return Err("Found 0 networks".into());
            }
            debug_eprintln!("Found {} network(s) for '{}'", networks.keys().len(), name);
            networks
        }
        None => return Err("Found no networks".into()),
    };

    // The documentation on https://docs.docker.com/engine/api/v1.44/#tag/Container/operation/ContainerInspect
    // is incomplete. There is another NetworkMode "default":
    // > which is bridge for Docker Engine, and overlay for Swarm.
    //
    // See: https://github.com/docker/docker-py/issues/986
    if network_mode == "default" && !networks.contains_key("default") {
        network_mode = "bridge"; // TODO add swarm support. Is this really used/needed?
    }

    // Get the end point settings for the network with the name in network_mode
    let end_point_settings = match networks.get(network_mode) {
        Some(end_point_settings) => end_point_settings,
        None => return Err(format!("Network '{network_mode}' not found").into()),
    };

    let ip_address = match &end_point_settings.ip_address {
        Some(ip_address) => {
            if ip_address.is_empty() {
                return Err("IP address is an empty string".into());
            }
            ip_address
        }
        None => return Err("Endpoint has no IP address".into()),
    };

    return match Ipv4Addr::from_str(ip_address) {
        Ok(ip) => {
            let id = match inspect_result.id.as_ref() {
                Some(id) => id,
                None => return Err("No Id".into()),
            };

            let mut aliases = vec![[id[..12].to_string(), SUFFIX.to_string()].join("")];

            if name.ne(query_stripped) {
                aliases.push([query_stripped.to_string(), SUFFIX.to_string()].join(""))
            }

            Ok(Some(Host {
                name: [name.to_string(), SUFFIX.to_string()].join(""),
                addresses: Addresses::V4(vec![ip]),
                aliases,
            }))
        }
        Err(_e) => {
            return Err(format!("Failed to parse IP address '{ip_address}': {_e}").into());
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Mock, Server, ServerOpts};
    use mocktopus::mocking::{MockResult, Mockable};

    #[test]
    fn test_get_host_by_name() {
        assert_eq!(
            DockerNG::get_host_by_name(".foo", AddressFamily::IPv4),
            Response::NotFound
        );

        assert_eq!(
            DockerNG::get_host_by_name("foo.docker", AddressFamily::IPv6),
            Response::NotFound
        );

        assert_eq!(
            DockerNG::get_host_by_name(".docker", AddressFamily::IPv4),
            Response::NotFound
        );

        let (_server, _mocks) = init_mocking_features();

        assert_eq!(
            DockerNG::get_host_by_name("sunny-default-bridge.docker", AddressFamily::IPv4),
            Response::Success(Host {
                name: "sunny-default-bridge.docker".to_string(),
                aliases: vec!["c0ffeec0ffee.docker".to_string()],
                addresses: Addresses::V4(vec![Ipv4Addr::new(172, 29, 0, 2)]),
            })
        );

        assert_eq!(
            DockerNG::get_host_by_name(
                "sunny-default-bridge-container-subdomains-allowed.docker",
                AddressFamily::IPv4
            ),
            Response::Success(Host {
                name: "sunny-default-bridge-container-subdomains-allowed.docker".to_string(),
                aliases: vec!["c0ffeec0ffee.docker".to_string()],
                addresses: Addresses::V4(vec![Ipv4Addr::new(172, 29, 0, 2)]),
            })
        );

        assert_eq!(
            DockerNG::get_host_by_name(
                "mega.very.sunny-default-bridge-container-subdomains-allowed.docker",
                AddressFamily::IPv4
            ),
            Response::Success(Host {
                name: "sunny-default-bridge-container-subdomains-allowed.docker".to_string(),
                aliases: vec![
                    "c0ffeec0ffee.docker".to_string(),
                    "mega.very.sunny-default-bridge-container-subdomains-allowed.docker"
                        .to_string()
                ],
                addresses: Addresses::V4(vec![Ipv4Addr::new(172, 29, 0, 2)]),
            })
        );

        assert_eq!(
            DockerNG::get_host_by_name("rainy-404.docker", AddressFamily::IPv4),
            Response::NotFound
        );

        assert_eq!(
            DockerNG::get_host_by_name("rainy-no-name.docker", AddressFamily::IPv4),
            Response::Unavail
        );

        assert_eq!(
            DockerNG::get_host_by_name("rainy-no-network-mode.docker", AddressFamily::IPv4),
            Response::Unavail
        );

        assert_eq!(
            DockerNG::get_host_by_name("rainy-network-mode-none.docker", AddressFamily::IPv4),
            Response::NotFound
        );

        assert_eq!(
            DockerNG::get_host_by_name("rainy-network-mode-host.docker", AddressFamily::IPv4),
            Response::NotFound
        );

        assert_eq!(
            DockerNG::get_host_by_name("rainy-network-mode-container.docker", AddressFamily::IPv4),
            Response::NotFound
        );

        assert_eq!(
            DockerNG::get_host_by_name("rainy-zero-networks.docker", AddressFamily::IPv4),
            Response::Unavail
        );

        assert_eq!(
            DockerNG::get_host_by_name("rainy-no-networks.docker", AddressFamily::IPv4),
            Response::Unavail
        );

        assert_eq!(
            DockerNG::get_host_by_name("rainy-network-not-exists.docker", AddressFamily::IPv4),
            Response::Unavail
        );

        assert_eq!(
            DockerNG::get_host_by_name("rainy-ip-address-empty.docker", AddressFamily::IPv4),
            Response::Unavail
        );

        assert_eq!(
            DockerNG::get_host_by_name("rainy-no-ip-address.docker", AddressFamily::IPv4),
            Response::Unavail
        );

        assert_eq!(
            DockerNG::get_host_by_name("rainy-unparseable-ip-address.docker", AddressFamily::IPv4),
            Response::Unavail
        );

        assert_eq!(
            DockerNG::get_host_by_name("rainy-no-id.docker", AddressFamily::IPv4),
            Response::Unavail
        );

        clear_mocking_features();
    }

    /*
     * Returns a server and its mocks based on https://github.com/lipanski/mockito
     */
    fn init_mocking_features() -> (Server, [Mock; 17]) {
        let mut server = Server::new_with_opts(ServerOpts {
            assert_on_drop: true,
            ..Default::default()
        });

        let url = server.url();

        // Mock it
        get_docker_uri.mock_safe(move || MockResult::Return(url.to_owned()));

        let _version_mock = server
            .mock("GET", "/version")
            .expect(16)
            .with_body_from_file("tests/resources/v1.44/version.body")
            .create();

        let _inspect_mock_sunny_default_bridge = server
            .mock("GET", "/v1.44/containers/sunny-default-bridge/json")
            .with_body_from_file("tests/resources/v1.44/containers/sunny-default-bridge/json.body")
            .create();

        let _inspect_mock_mega_very_sunny_default_bridge_container_subdomains_allowed = server
            .mock("GET", "/v1.44/containers/mega.very.sunny-default-bridge-container-subdomains-allowed/json")
            .with_status(404)
            .with_body_from_file("tests/resources/v1.44/containers/mega.very.sunny-default-bridge-container-subdomains-allowed/json.body")
            .create();

        let _inspect_mock_sunny_default_bridge_container_subdomains_allowed = server
            .mock("GET", "/v1.44/containers/sunny-default-bridge-container-subdomains-allowed/json")
            .expect(2)
            .with_body_from_file("tests/resources/v1.44/containers/sunny-default-bridge-container-subdomains-allowed/json.body")
            .create();

        let _inspect_mock_rainy_404 = server
            .mock("GET", "/v1.44/containers/rainy-404/json")
            .with_status(404)
            .with_body_from_file("tests/resources/v1.44/containers/rainy-404/json.body")
            .create();

        let _inspect_mock_rainy_no_name = server
            .mock("GET", "/v1.44/containers/rainy-no-name/json")
            .with_body_from_file("tests/resources/v1.44/containers/rainy-no-name/json.body")
            .create();

        let _inspect_mock_rainy_no_network_mode = server
            .mock("GET", "/v1.44/containers/rainy-no-network-mode/json")
            .with_body_from_file("tests/resources/v1.44/containers/rainy-no-network-mode/json.body")
            .create();

        let _inspect_mock_rainy_network_mode_none = server
            .mock("GET", "/v1.44/containers/rainy-network-mode-none/json")
            .with_body_from_file(
                "tests/resources/v1.44/containers/rainy-network-mode-none/json.body",
            )
            .create();

        let _inspect_mock_rainy_network_mode_host = server
            .mock("GET", "/v1.44/containers/rainy-network-mode-host/json")
            .with_body_from_file(
                "tests/resources/v1.44/containers/rainy-network-mode-host/json.body",
            )
            .create();

        let _inspect_mock_rainy_network_mode_container = server
            .mock("GET", "/v1.44/containers/rainy-network-mode-container/json")
            .with_body_from_file(
                "tests/resources/v1.44/containers/rainy-network-mode-container/json.body",
            )
            .create();

        let _inspect_mock_rainy_zero_networks = server
            .mock("GET", "/v1.44/containers/rainy-zero-networks/json")
            .with_body_from_file("tests/resources/v1.44/containers/rainy-zero-networks/json.body")
            .create();

        let _inspect_mock_rainy_no_networks = server
            .mock("GET", "/v1.44/containers/rainy-no-networks/json")
            .with_body_from_file("tests/resources/v1.44/containers/rainy-no-networks/json.body")
            .create();

        let _inspect_mock_rainy_network_not_exists = server
            .mock("GET", "/v1.44/containers/rainy-network-not-exists/json")
            .with_body_from_file(
                "tests/resources/v1.44/containers/rainy-network-not-exists/json.body",
            )
            .create();

        let _inspect_mock_rainy_ip_address_empty = server
            .mock("GET", "/v1.44/containers/rainy-ip-address-empty/json")
            .with_body_from_file(
                "tests/resources/v1.44/containers/rainy-ip-address-empty/json.body",
            )
            .create();

        let _inspect_mock_rainy_no_ip_address = server
            .mock("GET", "/v1.44/containers/rainy-no-ip-address/json")
            .with_body_from_file("tests/resources/v1.44/containers/rainy-no-ip-address/json.body")
            .create();

        let _inspect_mock_rainy_unparseable_ip_address = server
            .mock("GET", "/v1.44/containers/rainy-unparseable-ip-address/json")
            .with_body_from_file(
                "tests/resources/v1.44/containers/rainy-unparseable-ip-address/json.body",
            )
            .create();

        let _inspect_mock_rainy_no_id = server
            .mock("GET", "/v1.44/containers/rainy-no-id/json")
            .with_body_from_file("tests/resources/v1.44/containers/rainy-no-id/json.body")
            .create();

        (
            server,
            [
                _version_mock,
                _inspect_mock_sunny_default_bridge,
                _inspect_mock_mega_very_sunny_default_bridge_container_subdomains_allowed,
                _inspect_mock_sunny_default_bridge_container_subdomains_allowed,
                _inspect_mock_rainy_404,
                _inspect_mock_rainy_no_name,
                _inspect_mock_rainy_no_network_mode,
                _inspect_mock_rainy_network_mode_none,
                _inspect_mock_rainy_network_mode_host,
                _inspect_mock_rainy_network_mode_container,
                _inspect_mock_rainy_zero_networks,
                _inspect_mock_rainy_no_networks,
                _inspect_mock_rainy_network_not_exists,
                _inspect_mock_rainy_ip_address_empty,
                _inspect_mock_rainy_no_ip_address,
                _inspect_mock_rainy_unparseable_ip_address,
                _inspect_mock_rainy_no_id,
            ],
        )
    }

    fn clear_mocking_features() {
        get_docker_uri.clear_mock();
    }
}
