# Connecting From Other Machines

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

launcher:
  server_name: "http://server ip:21065"
```

If you need to change the ports our servers are using, add a `port` key to any of the above sections. The default port range is 21057-21067 over TCP.

It's recommended to serve Kawari with an HTTPS reverse proxy if you're sending data over an untrusted network.
