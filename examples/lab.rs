//! Port of the README walkthrough of the Python library.
//!
//! ```text
//! cargo run --example lab -- http://localhost:3080 API_TEST
//! ```

use std::sync::Arc;

use gns3fy_rs::{Gns3Connector, Lookup, Project};

#[tokio::main]
async fn main() -> gns3fy_rs::Result<()> {
    let mut args = std::env::args().skip(1);
    let url = args.next().unwrap_or_else(|| "http://localhost:3080".into());
    let project_name = args.next().unwrap_or_else(|| "API_TEST".into());

    // Define the server object to establish the connection
    let server = Arc::new(Gns3Connector::new(&url)?);
    println!("GNS3 server: {:?}", server.get_version().await?);

    // Show the available projects on the server
    for p in server.projects_summary().await? {
        println!("{p}");
    }

    println!();

    // Load a lab and retrieve its information
    let mut lab = Project::with_connector(server.clone()).with_name(project_name);
    if !lab.get().await? {
        lab.create().await?;
    }

    println!(
        "Name: {:?} -- Status: {:?} -- Auto closed?: {:?}",
        lab.name, lab.status, lab.auto_close
    );
    lab.open().await?;
    println!("Stats: {:?}", lab.stats);

    for node in lab.nodes_summary().await? {
        println!("{node}");
    }
    for link in lab.links_summary().await? {
        println!("{link}");
    }

    // Act on a single node
    if let Some(node) = lab.get_node_mut(Lookup::Name("alpine-1")).await? {
        node.stop().await?;
        println!("{:?}", node.status);
        node.start().await?;
        println!("{:?}", node.status);
    }
    Ok(())
}
