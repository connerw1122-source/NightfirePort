// Bulk-export the decompiled C of every function in the current program to
// <outRoot>/<programName>/<entry>_<name>.c, plus <outRoot>/<programName>/index.tsv
// (entry, name, size, file). Agents grep this tree instead of driving the Ghidra GUI.
// Usage (headless): -postScript ExportDecomp.java <outRoot>
// @category NightfirePort

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.FunctionIterator;

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;

public class ExportDecomp extends GhidraScript {
    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        if (args.length < 1) {
            printerr("usage: ExportDecomp.java <outRoot>");
            return;
        }
        String program = currentProgram.getName().replaceAll("[^A-Za-z0-9_.-]", "_");
        File dir = new File(args[0], program);
        dir.mkdirs();

        DecompInterface dec = new DecompInterface();
        dec.openProgram(currentProgram);
        int ok = 0, failed = 0;
        try (PrintWriter index = new PrintWriter(new FileWriter(new File(dir, "index.tsv")))) {
            index.println("entry\tname\tsize\tfile\tstatus");
            FunctionIterator it = currentProgram.getFunctionManager().getFunctions(true);
            while (it.hasNext() && !monitor.isCancelled()) {
                Function f = it.next();
                String entry = String.format("%08x", f.getEntryPoint().getOffset());
                String name = f.getName().replaceAll("[^A-Za-z0-9_.-]", "_");
                String file = entry + "_" + name + ".c";
                long size = f.getBody().getNumAddresses();
                DecompileResults r = dec.decompileFunction(f, 60, monitor);
                String status;
                if (r != null && r.decompileCompleted() && r.getDecompiledFunction() != null) {
                    try (PrintWriter w = new PrintWriter(new FileWriter(new File(dir, file)))) {
                        w.println("// " + program + " " + f.getEntryPoint() + " " + f.getName() + " size=" + size);
                        w.print(r.getDecompiledFunction().getC());
                    }
                    status = "ok";
                    ok++;
                } else {
                    status = "failed";
                    failed++;
                }
                index.println(entry + "\t" + f.getName() + "\t" + size + "\t" + file + "\t" + status);
            }
        } finally {
            dec.dispose();
        }
        println("ExportDecomp " + program + ": ok=" + ok + " failed=" + failed);
    }
}
