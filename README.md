# Holochain HTTP Gateway

[![Crate](https://img.shields.io/crates/v/holochain_http_gateway.svg)](https://crates.io/crates/holochain_http_gateway)
[![API Docs](https://docs.rs/holochain_http_gateway/badge.svg)](https://docs.rs/holochain_http_gateway)
[![Discord](https://img.shields.io/badge/Discord-blue.svg?style=flat-square)](https://discord.gg/k55DS5dmPH)

The Holochain HTTP Gateway provides a way to bridge from the web2 world into
Holochain. Read the [spec](./spec.md) for more details.

## Compatibility

| Holochain Version | HTTP Gateway Version |
| ----------------- | -------------------- |
| 0.4.x             | 0.1.x                |
| 0.5.x             | 0.2.x                |
| 0.6.x             | 0.3.x                |
| 0.7.x             | 0.4.x                |

## Running HTTP Gateway locally

### Prerequisites

Enter the Nix development shell. It provides the versions of Rust, Holochain,
and the `hc` CLI that are compatible with this checkout:

```bash
nix develop
```

Run the commands below from the repository root. Open each new terminal with
`nix develop` before continuing in it.

### Instructions

Clean the Holochain conductor sandbox with:

```bash
hc sandbox clean
```

Create a new sandboxed conductor with the lair keystore running in the
Holochain process, entering a passphrase when prompted:

```bash
hc sandbox create --in-process-lair
```

Run the sandboxed conductor. Enter a new passphrase when prompted and leave the
conductor running in this terminal:

```bash
hc sandbox run
```

Make note of the `admin_port`:

```console
machine:hc-http-gw$ hc sandbox run
hc-sandbox: Conductor launched #!0 {"admin_port":41191,"app_ports":[]}
```

In a second terminal, enter the Nix development shell and set `ADMIN_PORT` to
the value printed by `hc sandbox run`:

```bash
nix develop
export ADMIN_PORT=41191
```

In a separate terminal, also in the Nix `devShell`, we need to install the hApps that you want to use.
For example, we can use the test fixture (`fixture1`) but it needs to be built
first with the `package` script:

```bash
./fixture/package.sh
```

Install and enable the test fixture through the conductor's admin interface:

```bash
hc client call --port "$ADMIN_PORT" install-app \
  fixture/package/happ1/fixture1.happ
```

Run the gateway in the same terminal. The configuration connects it to the
conductor and permits HTTP calls to `coordinator1/get_all_1` in `fixture1`:

```bash
HC_GW_ADMIN_WS_URL="ws://localhost:$ADMIN_PORT" \
  HC_GW_ALLOWED_APP_IDS="fixture1" \
  HC_GW_ALLOWED_FNS_fixture1="coordinator1/get_all_1" \
  cargo run
```

In a third terminal, enter the Nix development shell and check the health
endpoint:

```bash
nix develop -c curl -i localhost:8090/health
```

```console
machine:hc-http-gw$ curl -i localhost:8090/health
HTTP/1.1 200 OK
content-type: text/plain; charset=utf-8
content-length: 2
date: Wed, 12 Mar 2025 15:25:13 GMT

Ok⏎
```

For the next steps, set `ADMIN_PORT` again and list the installed DNA hashes:

```bash
export ADMIN_PORT=41191
hc client call --port "$ADMIN_PORT" list-dnas
```

```console
machine:hc-http-gw$ hc client call --port "$ADMIN_PORT" list-dnas
["uhC0kwgZaQK05lgFwcYb_LrtAXTAckaS41nxNVO_zRMdpsuAeA0uN"]
```

Copy the returned value into `DNA_HASH`:

```bash
export DNA_HASH=uhC0kwgZaQK05lgFwcYb_LrtAXTAckaS41nxNVO_zRMdpsuAeA0uN
```

Calling `get_all_1` through the gateway should now return an empty JSON array:

```bash
curl -i \
  "localhost:8090/$DNA_HASH/fixture1/coordinator1/get_all_1"
```

```console
machine:hc-http-gw$ curl -i localhost:8090/uhC0kwgZaQK05lgFwcYb_LrtAXTAckaS41nxNVO_zRMdpsuAeA0uN/fixture1/coordinator1/get_all_1
HTTP/1.1 200 OK
content-type: text/plain; charset=utf-8
content-length: 2
date: Wed, 12 Mar 2025 15:26:00 GMT

[]
```

Add some data directly through the Holochain client. First, authorize zome
calls for the fixture. Enter a new passphrase when prompted; the client stores
the encrypted signing credentials in `.hc_auth` in the current directory:

```bash
hc client zome-call-auth --port "$ADMIN_PORT" fixture1
```

Create an entry, entering the same client passphrase when prompted:

```bash
hc client zome-call --port "$ADMIN_PORT" fixture1 "$DNA_HASH" \
  coordinator1 create_1 'null'
```

The created data can now be retrieved through the gateway:

```bash
curl -i \
  "localhost:8090/$DNA_HASH/fixture1/coordinator1/get_all_1"
```

```console
machine:hc-http-gw$ curl -i localhost:8090/uhC0kwgZaQK05lgFwcYb_LrtAXTAckaS41nxNVO_zRMdpsuAeA0uN/fixture1/coordinator1/get_all_1
HTTP/1.1 200 OK
content-type: text/plain; charset=utf-8
content-length: 50
date: Wed, 12 Mar 2025 17:54:50 GMT

[{"value":"create_1_2025-03-12T17:54:06.337428Z"}]
```

## Testing HTTP Gateway

Enter the Nix `devShell` with `nix develop` or make sure that you have
`rustup`, `perl`, `go`, and `holochain-cli` installed.

Then, build and package the test fixtures as hApps with the `package` script:

```bash
./fixture/package.sh
```

Then, simply run

```bash
cargo test
```

Or to run without the slower integration tests:

```bash
cargo test --lib --bins
```
