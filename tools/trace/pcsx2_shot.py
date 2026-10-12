"""Capture the real game's view at a chosen spot (our own code; plan v2 §3 image pairs).

Places Bond in the running PCSX2 game (player obj+0x30 position, +0x54 yaw, written through PINE), waits a few
in-game frames, then captures the PCSX2 window to a PNG with a Windows screen grab. Render the port from the same
spot with `nf-view <iso> <level> --bond x,y,z,yaw --frame N --shot out.png` and compare.

Usage: python -I tools/trace/pcsx2_shot.py <out.png> --pos x y z yaw [--wait frames]
The level must already be loaded (tools/trace/goto_level.py). Nothing is stored but the PNG you ask for.

Limit: the game collides Bond against his current cel (room). Teleporting into another room leaves that link
stale and he falls through the floor (07000001 gallery test). Use spots in the room Bond is already in, or walk
him there first, until the player's cel field is found and written too.
"""
import os
import struct
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pine import Pine  # noqa: E402

GAMESTATE = 0x2A3768
PLAYERS = 0x2D88E0
WINDOW = "007 - Nightfire"

GRAB = r"""
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System; using System.Runtime.InteropServices;
public class W {
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
}
'@
$p = Get-Process | Where-Object { $_.MainWindowTitle -like '*TITLE*' } | Select-Object -First 1
if (-not $p) { Write-Error 'PCSX2 game window not found'; exit 1 }
$h = $p.MainWindowHandle
[W]::SetForegroundWindow($h) | Out-Null
Start-Sleep -Milliseconds 400
$r = New-Object W+RECT; [W]::GetClientRect($h, [ref]$r) | Out-Null
$pt = New-Object W+POINT; [W]::ClientToScreen($h, [ref]$pt) | Out-Null
$w = $r.R - $r.L; $ht = $r.B - $r.T
$bmp = New-Object System.Drawing.Bitmap $w, $ht
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($pt.X, $pt.Y, 0, 0, $bmp.Size)
$bmp.Save('OUT', [System.Drawing.Imaging.ImageFormat]::Png)
Write-Output "captured ${w}x${ht}"
"""


def f2u(v):
    return struct.unpack("<I", struct.pack("<f", v))[0]


def main():
    a = sys.argv[1:]
    if not a or "--pos" not in a:
        print(__doc__)
        return
    out = os.path.abspath(a[0])
    i = a.index("--pos")
    x, y, z, yaw = (float(v) for v in a[i + 1:i + 5])
    wait = int(a[a.index("--wait") + 1]) if "--wait" in a else 20
    p = Pine()
    obj = p.read32(PLAYERS)
    # Hold the position for `wait` frames (the game's own movement and gravity would otherwise move him off it).
    start = p.read32(GAMESTATE + 0x34)
    while p.read32(GAMESTATE + 0x34) - start < wait:
        for k, v in enumerate((x, y, z)):
            p.write(obj + 0x30 + 4 * k, f2u(v), 4)
        p.write(obj + 0x54, f2u(yaw), 4)
        time.sleep(0.005)
    ps = GRAB.replace("TITLE", WINDOW).replace("OUT", out.replace("'", "''"))
    r = subprocess.run(["powershell", "-NoProfile", "-Command", ps], capture_output=True, text=True)
    print(r.stdout.strip() or r.stderr.strip())
    pos = [struct.unpack("<f", struct.pack("<I", p.read32(obj + 0x30 + 4 * k)))[0] for k in range(3)]
    print(f"level {p.read32(GAMESTATE + 0xC):08x}; Bond at {pos} yaw {yaw}; saved {out}")


if __name__ == "__main__":
    main()
