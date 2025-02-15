use crate::ebpf_portal::{ConnectionIdentifier, ParsedRawSocketPacket, Port};
use ockam_core::{Address, Route};
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::{Arc, RwLock};
use tokio::net::TcpListener;
use tokio::sync::mpsc::Sender;

/// Outlet registry
#[derive(Default, Clone)]
pub struct OutletRegistry {
    outlets: Arc<RwLock<HashMap<OutletKey, Outlet>>>,
}

impl OutletRegistry {
    /// Get outlet
    pub fn get_outlet(&self, src_ip: Ipv4Addr, src_port: Port) -> Option<Outlet> {
        self.outlets
            .read()
            .unwrap()
            .get(&OutletKey {
                dst_ip: src_ip,
                dst_port: src_port,
            })
            .cloned()
    }

    /// Add outlet
    pub fn add_outlet(
        &self,
        remote_worker_address: Address,
        internal_processor_address: Address,
        sender: Sender<ParsedRawSocketPacket>,
        dst_ip: Ipv4Addr,
        dst_port: Port,
    ) -> Outlet {
        let outlet_info = Outlet {
            remote_worker_address,
            internal_processor_address,
            sender,
            dst_ip,
            dst_port,
            connections1: Default::default(),
            connections2: Default::default(),
        };

        // TODO: eBPF Duplicates?
        self.outlets
            .write()
            .unwrap()
            .insert(OutletKey { dst_ip, dst_port }, outlet_info.clone());

        outlet_info
    }
}

#[derive(Hash, PartialEq, Eq)]
struct OutletKey {
    dst_ip: Ipv4Addr,
    dst_port: Port,
}

/// Outlet info
#[derive(Clone)]
pub struct Outlet {
    /// RemoteWorker Address
    pub remote_worker_address: Address,
    /// InternalProcessor Address
    pub internal_processor_address: Address,
    /// Sender to the InternalProcessor
    pub sender: Sender<ParsedRawSocketPacket>,
    /// Destination IP
    pub dst_ip: Ipv4Addr,
    /// Destination Port
    pub dst_port: Port,
    /// Same map with different key
    connections1: Arc<RwLock<HashMap<Port, Arc<OutletConnection>>>>,
    connections2: Arc<RwLock<HashMap<OutletConnectionKey, Arc<OutletConnection>>>>,
}

impl Outlet {
    /// Add mapping
    pub(crate) fn add_connection(&self, connection: Arc<OutletConnection>) {
        self.connections1
            .write()
            .unwrap()
            .insert(connection.assigned_port, connection.clone());
        self.connections2.write().unwrap().insert(
            OutletConnectionKey {
                identifier: connection.identifier.clone(),
                connection_identifier: connection.connection_identifier.clone(),
            },
            connection,
        );
    }

    /// Get Connection
    pub(crate) fn get_connection_internal(
        &self,
        assigned_port: Port,
    ) -> Option<Arc<OutletConnection>> {
        self.connections1
            .read()
            .unwrap()
            .get(&assigned_port)
            .cloned()
    }

    /// Get mapping
    pub(crate) fn get_connection_external(
        &self,
        identifier: Option<String>, // Identity
        connection_identifier: ConnectionIdentifier,
    ) -> Option<Arc<OutletConnection>> {
        self.connections2
            .read()
            .unwrap()
            .get(&OutletConnectionKey {
                identifier,
                connection_identifier,
            })
            .cloned()
    }
}

#[derive(Hash, PartialEq, Eq)]
struct OutletConnectionKey {
    identifier: Option<String>,
    connection_identifier: ConnectionIdentifier,
}

/// Outlet mapping
pub struct OutletConnection {
    /// Identity Identifier of the other side
    pub identifier: Option<String>,
    /// Unique connection Identifier
    pub connection_identifier: ConnectionIdentifier,
    /// Assigned port on our machine for a specific connection
    pub assigned_port: Port,
    /// Route to the other side PortalWorker
    pub return_route: Route, // TODO: Update it if the inlet updates the route
    /// To hold the port
    pub _tcp_listener: Arc<TcpListener>,
}
