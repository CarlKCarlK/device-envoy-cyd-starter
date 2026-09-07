#!/bin/sh

set -eu

if [ "${1:-}" = "--if-wsl" ]; then
    if ! grep -qi microsoft /proc/sys/kernel/osrelease 2>/dev/null; then
        exit 0
    fi
    busid=""
else
    if ! grep -qi microsoft /proc/sys/kernel/osrelease 2>/dev/null; then
        echo "attach-wsl is only needed inside WSL2." >&2
        exit 1
    fi
    busid="${1:-}"
fi

powershell_exe=/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe
if [ ! -x "$powershell_exe" ]; then
    echo "Windows PowerShell interop is unavailable in this WSL distribution." >&2
    exit 1
fi

script_path=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)/attach-wsl.ps1
script_windows_path=$(wslpath -w "$script_path")

"$powershell_exe" -NoProfile -ExecutionPolicy Bypass -File "$script_windows_path" "$busid"

attempt=0
while [ "$attempt" -lt 20 ]; do
    for port in /dev/ttyUSB* /dev/ttyACM*; do
        if [ -e "$port" ]; then
            if [ ! -r "$port" ] || [ ! -w "$port" ]; then
                wsl_exe=/mnt/c/Windows/System32/wsl.exe
                if [ ! -x "$wsl_exe" ] || [ -z "${WSL_DISTRO_NAME:-}" ]; then
                    echo "The CYD serial port is present at $port, but the current user cannot access it." >&2
                    exit 1
                fi

                echo "Granting the current WSL user access to $port..."
                "$wsl_exe" --distribution "$WSL_DISTRO_NAME" --user root -- \
                    chown "$(id -u):$(id -g)" "$port"
            fi

            if [ ! -r "$port" ] || [ ! -w "$port" ]; then
                echo "The CYD serial port is present at $port, but access could not be granted." >&2
                exit 1
            fi

            echo "CYD serial port available at $port"
            exit 0
        fi
    done
    attempt=$((attempt + 1))
    sleep 0.1
done

echo "The USB device was attached, but no /dev/ttyUSB* or /dev/ttyACM* port appeared." >&2
echo "Update WSL from Windows PowerShell with 'wsl --update', then reconnect the CYD." >&2
exit 1
