# Build the NightfirePort Ghidra project: the boot loader (SLUS_205.79), the Eurocom on-foot executable (ACTION.ELF,
# unstripped) and the EA driving executable (DRIVING.ELF, stripped), each its own
# program, analysed with the Emotion Engine processor, then bulk-export the decompiled C to work/decomp/<program>/.
# Prerequisite: `nf-extract elfs <iso> work/elf` has written the three ELFs (SHA-1s in docs/build-identity.md).
# Uses the Ghidra 12.1.4 + ghidra-emotionengine-reloaded install that already exists under RC2Recomp/tools
# (read-only use; nothing is written there).
# Stripped programs get the Aggressive Instruction Finder (pre) and pointer-target function creation (post): on RC2
# overlays that took level 0 from 180 to 3,722 functions (RC2Port WORKLOG 0008). Programs already exported are skipped.
param(
    [string]$ElfDir = '',
    [string]$Ghidra = 'C:\Users\Yoda\Desktop\RC2Recomp\tools\ghidra_12.1.4_PUBLIC',
    [switch]$Force
)
$ErrorActionPreference = 'Stop'
$root = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
if (-not $ElfDir) { $ElfDir = "$root\work\elf" }
$env:JAVA_HOME = 'C:\Program Files\Eclipse Adoptium\jdk-21.0.12.101-hotspot'
$proj = "$root\work\ghidra"
$decomp = "$root\work\decomp"
New-Item -ItemType Directory -Force $proj, $decomp | Out-Null
$headless = "$Ghidra\support\analyzeHeadless.bat"

function Import-Program([string]$path, [string[]]$extra) {
    $name = Split-Path $path -Leaf
    if (-not $Force -and (Test-Path "$decomp\$name\index.tsv")) { Write-Host "skip $name (already exported)"; return }
    $log = "$root\work\ghidra-$([IO.Path]::GetFileNameWithoutExtension($name)).log"
    & $headless $proj NightfirePort -import $path -overwrite -processor 'r5900:LE:32:default' `
        -scriptPath "$PSScriptRoot\scripts" @extra -postScript ExportDecomp.java $decomp -max-cpu 16 *> $log
    Write-Host ((Select-String $log -Pattern 'ExportDecomp ').Line)
}

$stripped = @('-preScript', 'EnableAggressiveFinder.java', '-postScript', 'CreatePointerFunctions.java')
Import-Program "$ElfDir\SLUS_205.79" $stripped
Import-Program "$ElfDir\ACTION.ELF" @()
Import-Program "$ElfDir\DRIVING.ELF" $stripped
Get-ChildItem $decomp -Directory | ForEach-Object {
    $idx = Import-Csv "$($_.FullName)\index.tsv" -Delimiter "`t"
    "{0}: {1} functions, {2} failed" -f $_.Name, $idx.Count, @($idx | Where-Object status -ne 'ok').Count
}
