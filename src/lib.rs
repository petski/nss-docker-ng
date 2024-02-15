#[macro_use]
extern crate lazy_static;
extern crate libnss;
extern crate debug_print;
extern crate docker_api;

use libnss::host::{AddressFamily, Addresses, Host, HostHooks};
use libnss::interop::Response;
use debug_print::debug_eprintln;
use std::net::{IpAddr, Ipv4Addr};
use std::error::Error;
use tokio;
use std::str::FromStr;
use docker_api::Docker;
use libnss::libnss_host_hooks;

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
async fn get_host_by_name(name: &str, family: AddressFamily) -> Result<Option<Host>, Box<dyn Error>> {
    // Check if the name ends with the expected suffix and if the address family is IPv4
    if !(
        name.ends_with(SUFFIX) &&
        family == AddressFamily::IPv4 && // TODO you no v6? See https://github.com/petski/nss-docker-ng/issues/3
        name.len() > SUFFIX.len()
    ) {
        return Ok(None);
    }

    // Initialize Docker API client
    let mut docker = Docker::new(DOCKER_URI)?;
    docker.adjust_api_version().await?;

    // Strip suffix from name
    let name_stripped = &name[..name.len() - SUFFIX.len()];

    // Fetch container information
    let inspect_result = match docker.containers().get(name_stripped).inspect().await {
        Ok(result) => result,
        Err(_e) => {
            debug_eprintln!("Failed to inspect container '{}': {}", name_stripped, _e);
            return Ok(None);
        }
    };

    let mut addresses = Vec::new();

    // Extract IP addresses from network settings
    if let Some(networks) = inspect_result.network_settings.and_then(|settings| settings.networks) {
        for (_network, end_point_settings) in networks {
            if let Some(ip_address) = end_point_settings.ip_address {
                if ip_address.len() == 0 {
                    debug_eprintln!("In '{}', IP address is an empty string", _network);
                    continue;
                }
                match Ipv4Addr::from_str(&ip_address) {
                    Ok(ip) => {
                        addresses.push(ip);
                        // For non-network search, pick the first
                        // TODO We could add some logic here instead of returning the first hit
                        //  i.e. using HostConfig.NetworkMode to determine the "default" network
                        //  See https://github.com/petski/nss-docker-ng/issues/4
                        break;
                    },
                    Err(_e) => {
                        debug_eprintln!("In '{}', failed to parse IP address '{}': {}", _network, ip_address, _e);
                    }
                }
            } else {
                debug_eprintln!("In '{}', IP address in empty", _network);
            }
        }
    }

    // If no addresses found, return None
    if addresses.is_empty() {
        return Ok(None);
    }

    Ok(Some(Host {
        name: name.to_string(),
        addresses: Addresses::V4(addresses),
        aliases: Vec::new(),
    }))
}
