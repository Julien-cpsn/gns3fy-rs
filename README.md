# gns3fy (Rust)

Rust wrapper around the [GNS3 server REST API](http://api.gns3.net/en/2.2/index.html) (GNS3 2.2+),
a port of the Python [`gns3fy`](https://github.com/davidban77/gns3fy) library. Use it to drive a GNS3
server from scripts, tooling and network CI/CD pipelines.

```toml
[dependencies]
gns3fy = { path = "." }
```

```rust
use std::sync::Arc;
use gns3fy::{Gns3Connector, Lookup, Project};

fn main() -> gns3fy::Result<()> {
    let server = Arc::new(Gns3Connector::new("http://localhost:3080")?);

    let mut lab = Project::with_connector(server.clone()).with_name("API_TEST");
    lab.get()?;   // project + stats + snapshots + drawings + nodes + links
    lab.open()?;

    for node in lab.nodes_summary()? { println!("{node}"); }
    for link in lab.links_summary()? { println!("{link}"); }

    if let Some(node) = lab.get_node_mut(Lookup::Name("alpine-1"))? {
        node.stop()?;
        node.start()?;
    }
    lab.create_link("alpine-1", "eth1", "IOU1", "Ethernet0/1")?;
    Ok(())
}
```

Run the bundled walkthrough with `cargo run --example lab -- http://localhost:3080 API_TEST`.

## Python → Rust mapping

| Python | Rust |
|---|---|
| `Gns3Connector(url, user, cred, verify, api_version)` | `Gns3Connector::new(url)` / `Gns3Connector::builder(url).user(..).cred(..).verify(..).api_version(..).build()` |
| `Project`, `Node`, `Link` (pydantic dataclasses) | `Project`, `Node`, `Link` structs, `Default` + `with_*` builders; share the connector with `Arc` |
| `name=` / `project_id=` / `template_id=` keyword pairs | `Lookup::Name(..)` / `Lookup::Id(..)` |
| `update(**kwargs)`, `create_template(**kwargs)` | take a `serde_json::Value` object: `node.update(json!({"x": 10}))?` |
| `*_summary(is_print=...)` | return `Vec` of typed rows; rows implement `Display` (same text the Python version printed) |
| validators on `node_type`, `console_type`, `status`, `link_type` | enums `NodeType`, `ConsoleType`, `NodeStatus`, `ProjectStatus`, `LinkType`; invalid values fail at deserialization |
| `ValueError` / `HTTPError` | `gns3fy::Error` (`InvalidInput`, `NotFound`, `Api{status,message}`, `MissingConnector`, `Http`, `Json`, `Io`) |
| `drawing_utils.generate_*_svg(...)` | `generate_*_svg(&Options { .. , ..Default::default() })` |

## Intentional differences from the Python library

- **No printing.** `create_node`, `create_link`, `create_snapshot`, `create_drawing`, `delete_*` and the
  `*_summary` methods return data instead of `print`ing. `create_*` return the created object.
- **`verify` is honored.** Python accepted `verify` but always sent `verify=False`. Here the default is still
  "do not verify", but `.verify(true)` really enables certificate checking.
- **Port-in-use / link matching is orientation-independent.** Python only compared `nodes[0]` with node A and
  `nodes[1]` with node B, so a link stored the other way round was missed. `create_link` now rejects any
  link that uses either port; `delete_link` finds the link in either orientation.
- **Missing objects are `NotFound` errors** where Python crashed with `IndexError`/`TypeError`/`AttributeError`
  (unknown node name, unknown project name, unknown template...).
- **No request timeout by default** (as in Python); set one with `.timeout(..)` on the builder.
- Connector-level calls (`get_projects`, `get_templates`, `get_computes`, ...) return raw `serde_json::Value`
  like the Python dicts; `Project`/`Node`/`Link` give you typed access.
- `nodes_inventory` returns a name-sorted `BTreeMap` (Python used insertion order).

## Tests

`cargo test` runs unit tests, doc-tests and integration tests against an in-process mock GNS3 server
(`tests/common`) using the original Python fixtures (`tests/data`). No GNS3 server is needed.
