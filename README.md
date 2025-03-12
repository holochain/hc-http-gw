# Holochain HTTP Gateway

[![Crate](https://img.shields.io/crates/v/holochain_http_gateway.svg)](https://crates.io/crates/holochain_http_gateway)
[![API Docs](https://docs.rs/holochain_http_gateway/badge.svg)](https://docs.rs/holochain_http_gateway)
[![Discord](https://img.shields.io/badge/Discord-blue.svg?style=flat-square)](https://discord.gg/k55DS5dmPH)

The Holochain HTTP Gateway for providing a way to bridge from the web2 world into Holochain

## Running HTTP Gateway locally

### Prerequisites

Enter the Nix `devShell` with `nix develop` or make sure that you have `rustup`
and `holochain-cli` installed.

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

Run the sandboxed conductor and enter the same passphrase as entered when
creating it:

```bash
hc sandbox run
```

Make note of the `admin_port`

```console
machine:hc-http-gw$ hc sandbox run
hc-sandbox: Conductor launched #!0 {"admin_port":41191,"app_ports":[]}
```

In a separate terminal, also in the Nix `devShell`, we need to install the hApps that you want to use.
For example, we can use the test fixture (`fixture1`) but it needs to be built
first with the `package` script:

```bash
./fixture/package.sh
```

Install the test fixture with:

```bash
hc sandbox call install-app fixture/package/happ1/fixture1.happ
```

Now run the gateway, setting the address of the admin websocket to `localhost`,
the port to the `"admin_port"` that was printed when running the sandboxed
conductor, the allowed apps to the installed fixture and the allowed
functions to the functions from the test fixture.

```bash
HC_GW_ADMIN_WS_URL="ws://localhost:41191" HC_GW_ALLOWED_APP_IDS="fixture1" HC_GW_ALLOWED_FNS_fixture1="coordinator1/get_all_1" cargo run
```

In another new terminal, also in the Nix `devShell`, check that we get a response to the health-check:

```bash
curl -i localhost:8090/health
```

```console
machine:hc-http-gw$ curl -i localhost:8090/health
HTTP/1.1 200 OK
content-type: text/plain; charset=utf-8
content-length: 2
date: Wed, 12 Mar 2025 15:25:13 GMT

Ok⏎
```

For the next steps, we'll need the DNA hash that the app was installed with.

```bash
hc sandbox call list-dnas
```

```console
machine:hc-http-gw$ hc sandbox call list-dnas
hc-sandbox: DNAs: [DnaHash(uhC0kwgZaQK05lgFwcYb_LrtAXTAckaS41nxNVO_zRMdpsuAeA0uN)]
```

The value you need is `uhC0kwgZaQK05lgFwcYb_LrtAXTAckaS41nxNVO_zRMdpsuAeA0uN`.

Calling the `get_all_1` function on the fixture using the DNA hash found above should 
now return an empty JSON array:

```bash
curl -i localhost:8090/uhC0kwgZaQK05lgFwcYb_LrtAXTAckaS41nxNVO_zRMdpsuAeA0uN/fixture1/coordinator1/get_all_1
```

```console
machine:hc-http-gw$ curl -i localhost:8090/uhC0kwgZaQK05lgFwcYb_LrtAXTAckaS41nxNVO_zRMdpsuAeA0uN/fixture1/coordinator1/get_all_1
HTTP/1.1 200 OK
content-type: text/plain; charset=utf-8
content-length: 2
date: Wed, 12 Mar 2025 15:26:00 GMT

[]⏎
```

Now let's add some data. First, authorise the zome call:

```bash
hc sandbox zome-call-auth fixture1
```

and create the data:

```bash
hc sandbox zome-call fixture1 uhC0kwgZaQK05lgFwcYb_LrtAXTAckaS41nxNVO_zRMdpsuAeA0uN coordinator1 create_1 'null'
```

Now the created data can be retrieved with

```bash
curl -i localhost:8090/uhC0kwgZaQK05lgFwcYb_LrtAXTAckaS41nxNVO_zRMdpsuAeA0uN/fixture1/coordinator1/get_all_1
```

```console
machine:hc-http-gw$ curl -i localhost:8090/uhC0kwgZaQK05lgFwcYb_LrtAXTAckaS41nxNVO_zRMdpsuAeA0uN/fixture1/coordinator1/get_all_1
HTTP/1.1 200 OK
content-type: text/plain; charset=utf-8
content-length: 50
date: Wed, 12 Mar 2025 17:54:50 GMT

[{"value":"create_1_2025-03-12T17:54:06.337428Z"}]⏎
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
