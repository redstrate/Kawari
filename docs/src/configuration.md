# Configuration

The config file for Kawari is named `config.yaml`, using the [YAML format](https://en.wikipedia.org/wiki/YAML).

## Game Path

By default Kawari will attempt to find an existing game install for you, but this won't always work. For example, if you installed the game on a different hard drive.

```yaml
filesystem:
    game_path: C:\Program Files (x86)\SquareEnix\FINAL FANTASY XIV - A Realm Reborn\game
```

## Connecting From Other Machines

By default, Kawari is not set up to accept connections from other machines on your network or from the internet. It's very simple to set up by changing your config:

```yaml
frontier:
  server_name: "http://server ip:21058"

lobby:
  server_name: "server ip"

login:
  server_name: "http://server ip:21060"

patch:
  server_name: "http://server ip:21061"

web:
  server_name: "http://server ip:21062"

world:
  server_name: "server ip"
```

If you need to change the ports our servers are using, add a `port` key to any of the above sections. The default port range is 21057-21067 over TCP.

It's recommended to serve Kawari with an HTTPS reverse proxy if you're sending data over an untrusted network.
