## HTTP interface

The gateway responds to HTTP GET requests with the following URL format:

```text
http://{host}/{dna-hash}/{coordinator-identifier}/{zome-name}/{function-name}?payload={payload}
```

Where the `dna-hash` is the base64 url encoded DNA hash of the DHT to retrieve data from, the `coordinator-identifier`, 
`zome-name` and `function-name` identify the zome function to invoke, and the `payload` query parameter is base64 url 
encoded JSON to be used as the zome call payload.

The `coordinator-identifier` is a way to ensure that the request is routed to a coordinator zome that has the expected 
interface. In the first iteration of the gateway, it is recommended that hApps are installed with a UUID or any UTF-8 
encoded string with at most 100 characters and this is used to identify the app to call. In the future, when Holochain 
properly supports app updates, this could be a coordinator hash. That would require Holochain exposing some concept of 
lineage so that newer coordinators that fulfill the interface of older ones, can be targeted.

## Status codes

| code | when?                                                                                                       | payload                                                                                                                                                           |
|------|-------------------------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 200  | Valid request and zome call succeeds                                                                        | JSON encoded zome call response                                                                                                                                   |
| 400  | Request is malformed                                                                                        | JSON message with an `error` field that contains a string explaining the problem.                                                                                 |
| 403  | The request appears valid but would require access to an app or function that is not exposed by the gateway | JSON message with an `error` field that describes the resource that the request wasn't allowed to access                                                          |
| 404  | The request is either for an unknown path or a resource we can't find like no app matching the `dna-hash`   | JSON message with an `error` field that contains a string explaining what resource wasn't found                                                                   |
| 405  | For any request to valid paths that doesn't use the GET method                                              | -                                                                                                                                                                 |
| 500  | For any internal error                                                                                      | JSON error response with an `error` field with a hard-coded string for conductor errors or the zome error message if this was an error raised by the target hApp. |

## Configuration

The HTTP gateway accepts configuration from environment variables.

| Environment variable       | Purpose                                                                                                                                      | Example                           |
|----------------------------|----------------------------------------------------------------------------------------------------------------------------------------------|-----------------------------------|
| HC_GW_ADMIN_WS_URL         | The websocket URL for Holochain's admin interface                                                                                            | `ws://localhost:8888`             |
| HC_GW_PAYLOAD_LIMIT_BYTES  | The maximum size for payloads, in bytes. This provides a limit on length of the URL that the gateway must process. (Default: `10240 (10kb)`) | `10240`                           |
| HC_GW_ALLOWED_APP_IDS      | Comma separated list of installed app ids that the gateway is allowed to access. If this var is not set, no apps are exposed.                | `mewsfeed,zipzap`                 |
| HC_GW_ALLOWED_FNS_{app-id} | Comma separated list of zome-scoped functions that the gateway is allowed to invoke for a given app.                                         | `main/list_mews,main/count_likes` |
| HC_GW_MAX_APP_CONNECTIONS  | The maximum number of app websocket connections that the gateway will maintain, one per allowed app. (Default: `50`)                         | `30`                              |
| HC_GW_ZOME_CALL_TIMEOUT_MS | Timeout in milliseconds for zome calls (Default: `10000` = 10s)                                                                              | 30000                             |

One `HC_GW_ALLOWED_FNS_{app-id}` variable must be set per allowed app id. For example `HC_GW_ALLOWED_FNS_mewsfeed=<zome function list>`.

The variable `HC_GW_ALLOWED_FNS_{app-id}` should permit `*` for users who don't wish to restrict access to the apps on 
the target Holochain conductor. Note that the gateway is doing nothing else to restrict access to functions that do 
write data, so opting out of this mechanism is **not** recommended.

## Request processing

### Validate the request

On receiving a request, the following must be checked:

- The DNA hash string must decode to a valid DNA hash.
- The coordinator identifier, zome name and function name must be valid UTF-8 and at most 100 characters long.
- The function name must be included in the list of allowed functions for the app.
- The payload length must be within the configured limit.
- The payload must be valid Base64 url encoded and decode to valid JSON.

### Identify the app to call

The gateway should use a single admin websocket to make requests to Holochain. Either use the existing connection or 
open a new one if the websocket is closed.

When receiving a valid request, the gateway should first check its cached list of installed apps. If the app isn't 
found in the cache, call the `ListApps` operation on Holochain with the filter set to only return running apps. The 
response should be stored in the cache for future requests.

The gateway must identify apps that match both the DNA hash specified in the request and have an installed app ID
matching the `coordinator-identifier`. It must then check if the app ID is in the list of allowed apps configured in 
`AllowedAppIds`. If multiple matching apps are found, an error should be returned since a unique app cannot be 
determined.

If no app was found in the initial check but the cache was repopulated, the search is performed again. If the app still 
isn't found, or if it's found but not in the allowed list, an appropriate error must be returned.

### Connect to Holochain to make app calls

The gateway uses its admin API connection to list app interfaces with the `ListAppInterfaces` request. It looks for an 
app interface that permits calls from the gateway or any location, and from the target app or any.

If a matching interface cannot be found, a new interface must be provisioned using the admin API request 
`AttachAppInterface`, and the `hc-http-gw` as an origin and with no app id specified.

Using either the discovered or created app interface, a app connection is established. This is done by issuing a 
connection token from the admin API with `IssueAppAuthenticationToken`. This token is then used to open an app 
connection for the selected app. 

After establishing the connection, the gateway authorizes signing credentials for each cell in the app and stores them
locally. 

> NOTE: The HTTP gateway uses a `ClientAgentSigner` from the [Rust client](https://github.com/holochain/holochain-client-rust)
to manage signing credentials for zome calls, instead of connecting to Holochain's keystore directly. The gateway then
uses its admin API connection to authorize these credentials for each cell via `authorize_signing_credentials`.
The granted functions are set according to the value of `HC_GW_ALLOWED_FNS_{app-id}`, either as All or a specific list
of functions.

This app connection is cached but the gateway closes older connections when needed to protect resources. How many 
connections the gateway will maintain is determined by `HC_GW_MAX_APP_CONNECTIONS`. If an errors occurs when making
zome calls that suggests the websocket connection is no longer valid, the gateway must attempt to reconnect. A single
reconnection attempt is made per HTTP request. If reconnecting fails, the gateway must return an error.

The gateway may cache the port of the selected app interface. App interfaces on Holochain are not guaranteed to use the
same port across restarts, so the gateway must be prepared to re-discover the port if a connection attempt fails. This
means that when caching the app port, the gateway must make up to two reconnection attempts before returning an error.

### Make the zome call

The request payload will already be in a JSON format because this was checked when receiving the request. 
Transcode the request to msgpack, using the `ExternIO` (serialized bytes) type from Holochain.

The target cell ID must be found from the app info discovered by searching listed apps. The cell ID is selected from
the app's provisioned cells by matching the input DNA hash.

The gateway dispatches the zome call to Holochain using the app API connection opened above, using a `CallZome` request
targeting the cell ID, zome name, function name and the provided payload.

On completion of the request, any errors are handled and converted to an HTTP 500 response. If the request succeeds 
then the `ExternIO` that is returned must be transcoded from msgpack to JSON and passed back with an HTTP 200 status.
