// Pre-analysis: enable Ghidra's Aggressive Instruction Finder. The level overlay ELFs carry no symbols, so without
// it Ghidra only finds code reachable from the entry point (measured: 180 functions on level 0).
// Usage (headless): -preScript EnableAggressiveFinder.java
// @category NightfirePort

import ghidra.app.script.GhidraScript;

public class EnableAggressiveFinder extends GhidraScript {
    @Override
    public void run() throws Exception {
        setAnalysisOption(currentProgram, "Aggressive Instruction Finder", "true");
        println("EnableAggressiveFinder: Aggressive Instruction Finder = " +
                getCurrentAnalysisOptionsAndValues(currentProgram).get("Aggressive Instruction Finder"));
    }
}
