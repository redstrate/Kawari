# Kawari

A substitute for a few official servers such as “ffxiv.com” and “square-enix.com”. It’s still early in development, but it can already emulate the basic login flow.

**Notice:** This does not allow you to download copyrighted game files, or circumvent the copyright protections enacted by Square Enix.

## Components

* Web
  * A simple website used for account management and other misc features.
* Admin
  * The admin panel for configuring the multitude of servers.
* [Frontier](https://docs.xiv.zone/server/frontier/)
  * Handles gate status requests.
* [Login](https://docs.xiv.zone/server/login/)
  * Handles logging in and giving a SID.
* [Patch](https://docs.xiv.zone/server/patch/)
  * Handles checking if the client needs any patching.

## Running

Install [Rust](https://rust-lang.org) and then use the `run.sh` helper script in the repository. You can of course run each server individually.  

### Testing via launcher

Testing on a real launcher is not yet supported, the easiest way is through [Astra](https://github.com/redstrate/Astra) which allows you to plug in your own domains. Because of how the domains are set up, you can't simply plug them in though.

You will need some kind of reverse proxy because simply editing the `hosts` file will not work. Each server is behind a subdomain (like `frontier.square-enix.com`) and some services span multiple subdomains (such as `patch-bootver.square-enix.com` and `patch-gamever.square-enix.com`.) We will walk through using [Caddy](https://caddyserver.com/) for this purpose, but any reverse proxy will do.

First you need to edit your `hosts` file. Assuming you're using the default ports for each server, add the following:

```
127.0.0.1 ffxiv.local
127.0.0.1 admin.ffxiv.local
127.0.0.1 frontier.ffxiv.local
127.0.0.1 patch-bootver.ffxiv.local
127.0.0.1 patch-gamever.ffxiv.local
127.0.0.1 ffxiv-login.square.local
```

Then run Caddy from the repository's `Caddyfile`. You may need to run it as root because it binds to port 80:

```
sudo caddy run
```

And then in Astra, plug these domains like so into Developer Settings:

* `square.local` into the "SE Login Server"
* `ffxiv.local` into "SE Main Server"

Make sure to set the "preferred protocol" to "HTTP" as well because HTTPS will not work without more setup.

## License

This project is licensed under the [GNU Affero General Public License 3](LICENSE). Some code or assets may be licensed differently.
