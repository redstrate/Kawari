@echo off

(
    start "admin" cmd /C "kawari-admin.exe"
    start "frontier" cmd /C "kawari-frontier.exe"
    start "login" cmd /C "kawari-login.exe"
    start "patch" cmd /C "kawari-patch.exe"
    start "web" cmd /C "kawari-web.exe"
    start "lobby" cmd /C "kawari-lobby.exe"
    start "world" cmd /C "kawari-world.exe"
    start "launcher" cmd /C "kawari-launcher.exe"
    start "savedatabank" cmd /C "kawari-savedatabank.exe"
) | pause
