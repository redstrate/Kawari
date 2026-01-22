# macOS

This guide covers how to setup Kawari on macOS.

> [!NOTE]
> We only support the x86-64 architecture, because a pre-compiled library we require isn't available for ARM64 yet.

## Requirements

* Legally obtained copy of the game that's updated to a supported version
* Oodle Network Compression
    * You can download the [latest release of Oodle from this repository](https://github.com/WorkingRobot/OodleUE/blob/main/Engine/Plugins/Compression/OodleNetwork/Sdks/2.9.15/lib). Click the "Mac" folder, and download the "liboo2netmac64.a" file.

## Firewall

By default, macOS will prompt you to "Allow Incoming Network Connections" for each Kawari binary. It also resets each time the binary files changes.

Make sure you hit Allow, or else the servers will refuse to function. If you missed this dialog, its possible to add the binaries manually under Advanced Firewall settings.

## Ports

By default, macOS uses ports that may conflict with the default set of Kawari ports. Until we change our default ports, if you see a server panic with "Address already in use" that means you need to change that server's port.
    
## Compiling

We can't provide macOS binaries for a variety of reasons (a lacking system linker and security) so [compiling Kawari](source.md) is the only option for now.
