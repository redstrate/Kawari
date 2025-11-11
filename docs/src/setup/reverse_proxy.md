Kawari isn't very useful unless it's addressable to a launcher, so we have to setup a "reverse proxy". Our tutorial uses [Caddy](https://caddyserver.com/download) and our built-in configuration works for a local setup. Run this in Command Prompt inside of the Kawari folder:

```shell
/path/to/your/caddy.exe run --config resources/data/Caddyfile
```

This Caddyfile hosts several domains required for normal operation, such as `ffxiv.localhost` on port 80.
