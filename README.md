# Sayless

Sayless is a simple link shortening service.

## Features

- Simple link shortening (domain + `/l/` + 7-character id)
- Link deduplication, if the same link is requested, same id would be returned
- API permission control via tokens
- GDPR-compliant configurable IP recording
- Ability to block certain IPs to prevent malicious links, spam and abuse

### IP recording

This feature is optional. During development GDPR compliance was an explicit requirement. And so, this function is optional and when enabled, requires a retention period to be specified. It's not intended to be used for metrics, although you're not getting stopped from usingit that way by querying the database. The intended use is to check whether each link is malicious and detect spam and abuse. It is assumed that an external service would be taking care of this. Malicious link, spam and/or abuse would result in a strike being recorded for the IP that created the link. I am still not sure if only the hash of the IP should be recorded instead of plain IP. An excessive amount of strikes recorded on single IP address would result in this IP being blocked from creating new links.

### Token authorization system

This feature is optional. Tokens provide a way to limit link creation and data access. A token is a 44-character-long string that uses characters from base58 set. This results in a bit over 256^2 possible tokens. Each token has an expidation date. By default it is set to be 1 year away from creation time and no easy API for changing it is implemented yet but it is planned. Each token has this list of permissions (subject to change):

- Administrator permission. Grants permission to use everything without needing to specify each permission.
- Link creation permission. If link creation requires a token with valid permission, only clients that supply a token with this permission would be allowed to create shortened links.
- Token creation permission. This permission allows creating new tokens with any permissions (possible loophole).
- IP view permission. Allows seeing IPs of clients that created shortened links via API.

For security and ease of setup, a master token must be provided via `MASTER_TOKEN` environment variable if token system is enabled. It grants access to all functions of the service and cannot be disabled or removed. It can be a string of any length. Character set is limited to what can be used in a header value.

Authorization is performed via Authorization Bearer header.

### API description

Request type is GET unless specified otherwise.

#### `/l/:id`

Redirects to the link with this `id`. Uses response code 303 and the link is in Location header.

#### `/l/create`

Request type: POST. Accepts the link via request body. Returns 201 code on success and the shortened link in the form `/l/:id` via Location header. Optionally takes an Authorization Bearer token if link creation only by authorized users is configured

#### `/l/:id/info`

Gets information about a link with this `id`. The returned information is located in the response body as JSON:

- `"id"`: `id` of the link
- `"link"`: the link associated with this `id`
- `"hash"`: hex-formatted blake3 hash of the link
- `"created_at"`: date and time when this link was created
- `"created_by"`: IP address of the client that created this link. Only returned if the token auth is enabled and a valid token with ip view permission was provided.

#### `/l/tokens/create`

Request type: POST. Only available if token system is enabled. Creates a token with specified permissions. Returns status code 201 on success and the created token in the response body. Token permissions are provided via JSON payload in request body, all values default to `false`:

- `"admin_perm"` - Admin permission. This permission grants access for all permissions.
- `"create_link_perm"` - Permission to create links. Only has effect if link creation is configured to require a token with this permission.
- `"create_token_perm"` - Permission to create new tokens.
- `"view_ips_perm"` - Permission to view origin IPs of clients that created a link.

### Configuration

This service is configured via a config file and environment variables.

#### Environment variables

`dotenvy` is used to load envoronment variables from `.env` file, if it exists. Otherwise, environment variables need to be set via other means.

- `DATABASE_URL`: This environment variable controls what database the service would use. Sayless uses MySQL databases, such as MariaDB. The link must follow this format: `mysql://<user>:<password>@<host>:<port>/<database>`.
- `MASTER_TOKEN`: Only required to be set if token system is enabled. This token can be used to access all endpoints, for example creating new tokens. The length is not limited, but the character set is limited to what a header value can contain.

#### Configuration file

`config.toml` is used for service configuration. An example file with default values is provided in the repository.

- `port` - Required. Port thatthe web server is listening on.
- `max_strikes` - Optional. Default: `30`. Only used if ip recording is enabled. If the IP of a client that is trying to create a new link has number of strikes recorded that is higher than or equals to this number, the client would be rejected in link creation.
- `log_level` - Optional. Default: `"info"`. Sets log level. Possible log levels are, in increasing order of verbosity: `"error"`, `"warn"`, `"info"`, `"debug"`, `"trace"`.
- `[token_config]` - Optional table. If present (table header is enough), the token system is enabled.
  - `creation_requires_auth` - Optional. Default: `false`. If set to `true`, creating a shortened link would require providing a token with link creation permission.
- `[ip_recording]` - Optional table. If present (table header s enough), ip recording is enabled.
  - `retention period` - Optional. Default: `"2w"`. Sets the period for which the IPs would be stored in the database. Format is explained in the example config.
  - `retention_check_period` - Optional. Default: "0 0 * * *". Sets a schedule for when to check the IP addresses database for IPs which no longer need to be stored. Uses cronjob syntax.
