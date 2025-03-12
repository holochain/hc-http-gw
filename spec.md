## HTTP interface

The gateway responds to HTTP GET requests with the following URL format

```
http://<host>/<dna-hash>/<coordinator-identifier>/<zome-name>/<function-name>?payload=<payload>
```

Where the `dna-hash` is the base64url encoded DNA hash of the DHT to retrieve data from, the `coordinator-identifier`, `zome-name` and `function-name` identify the zome function to invoke, and the `payload` query parameter is base64url encoded JSON to be used as the zome call payload.

The `coordinator-identifier` is a way to ensure that the request is routed to a coordinator zome that has the expected interface. In the first iteration of the gateway, it is recommended that hApps are installed with a UUID or any UTF-8 encoded string with at most 100 characters long and this is used to identify the app to call. In the future, when Holochain properly supports app updates, this could be a coordinator hash. That would require Holochain exposing some concept of lineage so that newer coordinators that fulfill the interface of older ones, can be targeted.

| code | when?                                                                                                                                                   | payload                                                                                                                                                           |
| ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 200  | Valid request and zome call succeeds                                                                                                                    | JSON encoded zome call response                                                                                                                                   |
| 400  | Request is malformed                                                                                                                                    | JSON message with an `error` field that contains a string explaining the problem.                                                                                 |
| 403  | The request appears valid but would require access to an app or function that is not exposed by the gateway                                             | JSON message with an `error` field that describes the resource that the request wasn't allowed to access                                                          |
| 404  | The request is either for an unknown path or a resource we can't find like no app matching the `dna-hash` ~~OR the app being accessed isn't permitted~~ | JSON message with an `error` field that contains a string explaining what resource wasn't found                                                                   |
| 405  | For any request to valid paths that doesn't use the GET method                                                                                          |                                                                                                                                                                   |
| 500  | For any internal error, we should log and return a generic error                                                                                        | JSON error response with an `error` field with a hard-coded string for conductor errors or the zome error message if this was an error raised by the target hApp. |

## Configuration

The HTTP gateway accepts configuration from environment variables.

| Environment variable       | Purpose                                                                                                                                                                             | Example                           |
| -------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------- |
| HC_GW_ADMIN_WS_URL         | The websocket URL for Holochain's admin interface                                                                                                                                   | `ws://localhost:8888`             |
| HC_GW_PAYLOAD_LIMIT_BYTES  | The maximum size for payloads, in bytes. This provides a limit on length of the URL that the gateway must process. If this var is not set, `10240 (10kb)` will be the default value | `10240`                           |
| HC_GW_ALLOWED_APP_IDS      | Comma separated list of installed app ids that the gateway is allowed to access. If this var is not set, we assume that no apps are exposed.                                        | `mewsfeed,zipzap`                 |
| HC*GW_ALLOWED_FNS*<app-id> | Comma separated list of zome-scoped functions that the gateway is allowed to invoke for a given app.                                                                                | `main/list_mews,main/count_likes` |
| HC_GW_ZOME_CALL_TIMEOUT_MS | Timeout in milliseconds for zome calls (Default: 10000)                                                                                                                             | 30000                             |

One `HC_GW_ALLOWED_FNS_<app-id>` variable must be set per allowed app id. For example `HC_GW_ALLOWED_FNS_mewsfeed=<zome function list>`.

The variable `HC_GW_ALLOWED_FNS_<app-id>` should permit `*` for users who don't wish to restrict access to the apps on the target Holochain conductor. Note that the gateway is doing nothing else to restrict access to functions that do write data, so opting out of this mechanism is not recommended.

## Request processing

### Validate the request

On receiving a request, the request should be checked:

- The DNA hash string must decode to a valid DNA hash.
- The coordinator identifier and zome name must be valid UTF-8 and at most 100 characters long.
- The function name must be a valid function identifier (valid UTF-8 is good enough for now) and at most 100 characters long.
- The function name must be included in the list of allowed functions for the app.
- The payload length must be within the configured limit.
- The payload must be valid base64url and decode to valid JSON.

### Identify the app to call

The gateway uses a single admin websocket to make requests to Holochain. At this point, either use the existing connection or open a new one if the websocket is closed.

When receiving a valid request, the gateway first checks its cached list of installed apps. If the cache is empty or the app isn't found in the cache, it calls the `ListApps` operation on Holochain with the filter set to only return running apps. The response is stored in the cache for future requests.

The gateway identifies apps that match both the DNA hash specified in the request and have an installed app ID matching the `coordinator-identifier`. It then checks if the app ID is in the list of allowed apps configured in `AllowedAppIds`. If multiple matching apps are found, an error is returned since a unique app cannot be determined.

If no app is found in the initial check but the cache was populated, a fresh request to `ListApps` is made to update the cache, and the search is performed again. If the app still isn't found, or if it's found but not in the allowed list, an appropriate error is returned.

### Prepare zome call signing

At this point, we appear to have received a valid request that identifies an app running on Holochain. We should now try to issue signing credentials for ourselves.

The HTTP gateway uses ClientAgentSigner to manage signing credentials for zome calls, instead of using `sodoken` directly. The gateway then uses its admin API connection to authorize these credentials for each cell via authorize*signing_credentials. The granted functions are set according to the value of HC_GW_ALLOWED_FNS*<app-id>, either as All or a specific list of functions.

### Connect to Holochain to make app calls

The gateway uses its admin API connection to list app interfaces with the `ListAppInterfaces` request.

It looks for an app interface that permits calls from the gateway or any location, and from the target app or any.

If a matching interface cannot be found, a new interface is provisioned using the admin API request `AttachAppInterface`, the `hc-http-gw` as an origin and with no app id specified.

Using either the discovered or created app interface, a connection is established. This is done by issuing a connection token from the admin API with `IssueAppAuthenticationToken`. This token is then used to open an app connection for the selected app. After establishing the connection, the gateway authorizes signing credentials for each cell in the app and stores them in the client signer.

This app connection is cached but the gateway closes older connections when needed to protect resources. If errors occur, the gateway attempts to reconnect.

### Make the zome call

The request payload is already in the appropriate format as `ExternIO` (serialized bytes).

The gateway dispatches the zome call to Holochain using the app API connection opened above, using a `CallZome` request targeting the specified cell ID, zome name, and function name with the provided payload.

On completion of the request, any errors are handled and converted to an HTTP 500 response. If the request succeeds then the `ExternIO` that is returned is passed back with an HTTP 200 status.

### Limitations and future extension

- We are requiring direct access to Holochain's admin interface because we need to perform several operations against it. This will require either that the gateway is hosted on the same machine as the conductor or that traffic to the admin websocket is exposed some other way. Holochain will only listen on the loopback address for its admin API.
- We are not able to issue zome call signing keys that are usable across an app which requires some extra work to authorize the keypair with each cell we need to use it with. In most apps, this will just be one DNA that people want data from but we still have to provide code that is general. The mechamism [needs replacing](https://github.com/holochain/holochain/issues/4595) with something better suited to this common use-case.
- Storing signing keys in memory opens up surface area for compromise that Lair is designed to protect. We could consider replacing generating keys in memory with use of Lair for the gateway in the future. This makes it a little harder to operate but would increase security.
- We are not really enforcing "read-only". Any zome functions that are exposed through configuration will be called by the gateway without any knowledge of whether they write to the source chain. It is a best-effort by developers and the user operating the gateway that will prevent unintended side-effects. It is important to understand misconfiguration. This is a significant risk to the DHTs the gateway can access. In the future, Holochain might consider adding something like [this](https://github.com/holochain/holochain/issues/3009).
- There is no provided mechanism to re-use signing keys. This means that every time the gateway restarts, it will provision new signing keys and store them in Holochain. There is currently no delete operation for these. There is a soft delete, but nothing that will remove the data. A further reason that this mechanism doesn't suit the use-case.
- Error handling is intentionally quite limited. If Holochain's errors were simplified then we might be able to give some better feedback. Even as they are, it might turn out as we start to use this, that some errors would be useful to return to the user.
