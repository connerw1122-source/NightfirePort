# Start a ghidra-mcp headless server on the NightfirePort Ghidra project (loopback only, default port 8089: the registered bridge only reaches
# 127.0.0.1:8089 over TCP, so stop RC2's server first). Reuses the Ghidra + GhidraMCP build under RC2Recomp/tools read-only.
# The registered MCP bridge finds it with list_instances / connect_instance("NightfirePort").
# Usage: .\start-ghidra-mcp.ps1 [-Port 8090] [-Program ACTION.ELF]     (foreground; Ctrl+C to stop)
# Do not run build-project.ps1 while this server holds the project open.
param([int]$Port = 8089, [string]$Program = 'ACTION.ELF')

$root   = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
$tools  = 'C:\Users\Yoda\Desktop\RC2Recomp\tools'
$ghidra = "$tools\ghidra_12.1.4_PUBLIC"
$jar    = "$tools\ghidra-mcp\build\libs\GhidraMCP-7.0.0.jar"
$env:JAVA_HOME = 'C:\Program Files\Eclipse Adoptium\jdk-21.0.12.101-hotspot'

# Ghidra framework/feature/processor jars, plus installed extensions (Emotion Engine).
$jars = @($jar)
foreach ($pattern in 'Framework\*\lib\*.jar', 'Features\*\lib\*.jar', 'Processors\*\lib\*.jar', 'Extensions\*\lib\*.jar') {
    $jars += Get-ChildItem "$ghidra\Ghidra\$pattern" -ErrorAction SilentlyContinue | ForEach-Object FullName
}
$classpath = $jars -join ';'

& "$env:JAVA_HOME\bin\java.exe" -Xmx4g -XX:+UseG1GC `
    "-Dghidra.home=$ghidra" "-Dapplication.name=GhidraMCP" `
    -classpath $classpath com.xebyte.headless.GhidraMCPHeadlessServer `
    --port $Port --bind 127.0.0.1 `
    --project "$root\work\ghidra\NightfirePort.gpr" --program $Program
