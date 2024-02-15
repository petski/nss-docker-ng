#[macro_use]
extern crate lazy_static;
extern crate debug_print;
extern crate docker_api;
extern crate libnss;

use debug_print::debug_eprintln;
use docker_api::Docker;
use libnss::host::{AddressFamily, Addresses, Host, HostHooks};
use libnss::interop::Response;
use libnss::libnss_host_hooks;
use std::error::Error;
use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;
use tokio;

static SUFFIX: &'static str = ".docker";
static DOCKER_URI: &'static str = "unix:///var/run/docker.sock";

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
    let mut docker = Docker::new(DOCKER_URI)?;
    docker.adjust_api_version().await?;

    // Strip suffix from query
    let query_stripped = &query[..query.len() - SUFFIX.len()];

    // Fetch container information
    let inspect_result = match docker.containers().get(query_stripped).inspect().await {
        Ok(result) => result,
        Err(_e) => {
            debug_eprintln!("Failed to inspect container '{}': {}", query_stripped, _e);
            return Ok(None);
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
    let network_mode = match inspect_result.host_config.as_ref().and_then(|host_config| {
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

    // Get the end point settings for the network with the name in network_mode
    let end_point_settings = match networks.get(network_mode) {
        Some(end_point_settings) => end_point_settings,
        None => return Err(format!("Network '{}' not found", network_mode).into()),
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

    return match Ipv4Addr::from_str(&ip_address) {
        Ok(ip) => {
            let id = match inspect_result.id.as_ref() {
                Some(id) => id,
                None => return Err("No Id".into()),
            };

            let mut aliases = Vec::new();
            aliases.push([id[..12].to_string(), SUFFIX.to_string()].join(""));

            Ok(Some(Host {
                name: [name.to_string(), SUFFIX.to_string()].join(""),
                addresses: Addresses::V4(vec![ip]),
                aliases,
            }))
        }
        Err(_e) => {
            return Err(format!("Failed to parse IP address '{}': {}", ip_address, _e).into());
        }
    };
}
