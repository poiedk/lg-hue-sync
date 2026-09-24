#!/usr/bin/env bash
set -euo pipefail

TV_IP="${1:?Usage: ./scripts/probe_tv_compat.sh <tv-ip> [ssh-port]}"
SSH_PORT="${2:-22}"
SSH_OPTS=(-p "$SSH_PORT" -o StrictHostKeyChecking=accept-new -o ConnectTimeout=5)

ssh "${SSH_OPTS[@]}" root@"$TV_IP" 'sh -s' <<'REMOTE'
set -u

echo "=== OS / kernel ==="
cat /etc/os-release 2>/dev/null || true
cat /etc/starfish-release 2>/dev/null || true
uname -a 2>/dev/null || true
uname -m 2>/dev/null || true

echo
echo "=== libc ==="
getconf GNU_LIBC_VERSION 2>/dev/null || true
ldd --version 2>&1 | head -n 2 || true
for libc in /lib/libc.so.6 /usr/lib/libc.so.6; do
    if [ -e "$libc" ]; then
        echo "libc_path=$libc"
        if command -v strings >/dev/null 2>&1; then
            strings "$libc" 2>/dev/null | grep '^GLIBC_[0-9]' | sort -Vu | tail -n 8 || true
        fi
        break
    fi
done

echo
echo "=== capture libraries ==="
for path in /usr/lib/libdile_vt.so.0 /usr/lib/libhal_vt.so.2 /usr/lib/libvtcapture.so; do
    if [ -e "$path" ]; then
        ls -l "$path"
    else
        echo "missing: $path"
    fi
done

echo
echo "=== DILE symbols ==="
DILE=/usr/lib/libdile_vt.so.0
if [ -e "$DILE" ]; then
    if command -v nm >/dev/null 2>&1; then
        nm -D "$DILE" 2>/dev/null | grep -E 'DILE_VT_(CreateEx|Create|Start|Stop|Destroy|SetVideoFrameOutputDeviceDumpLocation|SetVideoFrameOutputDeviceOutputRegion|SetVideoFrameOutputDeviceState|GetVideoFrameBufferCapability|GetAllVideoFrameBufferProperty|GetCurrentVideoFrameBufferProperty|WaitVsync)' || true
    elif command -v readelf >/dev/null 2>&1; then
        readelf -Ws "$DILE" 2>/dev/null | grep -E 'DILE_VT_(CreateEx|Create|Start|Stop|Destroy|SetVideoFrameOutputDeviceDumpLocation|SetVideoFrameOutputDeviceOutputRegion|SetVideoFrameOutputDeviceState|GetVideoFrameBufferCapability|GetAllVideoFrameBufferProperty|GetCurrentVideoFrameBufferProperty|WaitVsync)' || true
    else
        echo "nm/readelf unavailable; symbol inspection skipped"
    fi
fi

echo
echo "=== runtime/service support ==="
command -v systemctl 2>/dev/null || true
systemctl --version 2>/dev/null | head -n 1 || true
command -v luna-send 2>/dev/null || true
test -r /dev/mem && echo "/dev/mem readable: yes" || echo "/dev/mem readable: no"
test -e /dev/video60 && ls -l /dev/video60 || echo "/dev/video60: not present at probe time"

echo
echo "=== probe complete (read-only) ==="
REMOTE
